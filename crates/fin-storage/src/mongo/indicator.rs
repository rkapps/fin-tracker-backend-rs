use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fin_domain::{
    dto::ticker_indicator_entity::TickerIndicatorEntity,
    tickers::{IndicatorSnapshot, IndicatorWindow, TickerIndicator},
};
use rustic_storage::core::{repository::Repository, search::SearchCriteria};
use serde_json::json;
use tracing::{debug, warn};

use crate::{
    mongo::MongoStorageService,
    service::{TickerIndicatorStorageService, TickerStorageService},
};
use anyhow::Result;

#[async_trait]
impl TickerIndicatorStorageService for MongoStorageService {
    async fn delete_ticker_indicators_before(&self, date: DateTime<Utc>) -> Result<()> {
        let criteria = SearchCriteria::new().lt("date", date);

        match self.manager.ticker_indicators().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.delete_many(Some(criteria)).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error getting TickerSentiment: {}", e));
            }
        }
    }

    async fn delete_ticker_indicators(&self, symbol: &str) -> Result<()> {
        let criteria = SearchCriteria::new().eq("symbol", symbol.to_uppercase());
        let Ok(repo) = self.manager.ticker_indicators().await else {
            return Err(anyhow::anyhow!(format!(
                "Error finding TickerIndicator for '{}'",
                symbol
            )));
        };

        let mut repo = repo.lock().await;
        repo.delete_many(Some(criteria)).await?;

        Ok(())
    }

    async fn get_ticker_indicators_by_symbols(
        &self,
        symbols: Vec<String>,
        n: Option<usize>,
    ) -> Result<Vec<TickerIndicatorEntity>> {
        debug!("symbols: {:?}", symbols);

        let n = n.unwrap_or(0);

        match self.manager.ticker_indicators().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;

                let pipeline = vec![
                    json!({ "$match": { "symbol": { "$in": &symbols } } }),
                    json!({ "$sort": { "symbol": 1, "date": -1 } }),
                    json!({ "$group": {
                        "_id": "$symbol",
                        "docs": { "$push": "$$ROOT" }
                    }}),
                    match n {
                        0 => json!({ "$project": {
                            "symbol": "$_id",
                            "records": "$docs",
                            "_id": 0
                        }}),
                        _ => json!({ "$project": {
                            "symbol": "$_id",
                            "records": { "$slice": ["$docs", n as i64] },
                            "_id": 0
                        }}),
                    },                    
                    json!({ "$unwind": "$records" }),
                    json!({ "$replaceRoot": { "newRoot": "$records" } }),                    
                    json!({ "$addFields": {
                        "rsi_14": { "$toDouble": "$values.rsi_14" },
                        "sma_50": { "$toDouble": "$values.sma_50" },
                        "sma_200": { "$toDouble": "$values.sma_200" },
                        "macd": { "$toDouble": "$values.macd" },
                        "macd_signal": { "$toDouble": "$values.macd_signal" },
                        "bb_upper": { "$toDouble": "$values.bb_upper" },
                        "bb_lower": { "$toDouble": "$values.bb_lower" },
                    }}),
                    json!({ "$project": {
                        "id" : 1,
                        "symbol": 1,
                        "date": 1,
                        "rsi_14": 1,
                        "sma_50": 1,
                        "sma_200": 1,
                        "macd": 1,
                        "macd_signal": 1,
                        "bb_upper": 1,
                        "bb_lower": 1,
                        "_id": 0
                    }}),
                ];
                let results = repo.aggregate(pipeline).await?;
                debug!("results: {:?}", results);

                let indicators: Vec<TickerIndicatorEntity> = results
                    .iter()
                    .filter_map(|v| match serde_json::from_value(v.clone()) {
                        Ok(i) => Some(i),
                        Err(e) => {
                            warn!(
                                "Failed to deserialize TickerIndicator: {} value: {:#?}",
                                e, v
                            );
                            None
                        }
                    })
                    .collect();
                debug!("results: {:?}", indicators);
                Ok(indicators)
            }
            Err(e) => Err(anyhow::anyhow!("Error getting TickerIndicator: {}", e)),
        }
    }

    async fn get_ticker_indicators(&self, symbol: &str) -> Result<Vec<TickerIndicator>> {
        let criteria = SearchCriteria::new()
            .eq("symbol", symbol.to_uppercase())
            .sort_asc("date");
        self.get_ticker_indicators_by_criteria(&criteria).await
    }

    async fn get_ticker_indicators_by_symbol(
        &self,
        symbol: &str,
        from_date: DateTime<Utc>,
    ) -> Result<Vec<TickerIndicator>> {
        let criteria = SearchCriteria::new()
            .eq("symbol", symbol.to_uppercase())
            .gte("date", from_date)
            .sort_asc("date");
        self.get_ticker_indicators_by_criteria(&criteria).await
    }

    async fn get_ticker_indicators_latest(&self, symbol: &str) -> Result<TickerIndicator> {
        let indicators = self.get_ticker_indicators_last_n(symbol, 1).await?;
        let indicator = indicators.first().unwrap();
        Ok(indicator.clone())
    }

    async fn get_ticker_indicators_last_n(
        &self,
        symbol: &str,
        n: usize,
    ) -> Result<Vec<TickerIndicator>> {
        let criteria = SearchCriteria::new()
            .eq("symbol", symbol)
            .sort_desc("date")
            .limit(n);
        self.get_ticker_indicators_by_criteria(&criteria).await
    }

    async fn get_ticker_indicators_map_by_sector(
        &self,
        sector: &str,
        from_date: DateTime<Utc>,
    ) -> Result<HashMap<String, Vec<TickerIndicator>>> {
        let tickers = self.get_ticker_by_sector(sector).await?;
        let symbols: Vec<String> = tickers.iter().map(|t| t.symbol.clone()).collect();

        debug!("Tickers for sector: {}-{:?}", sector, symbols);
        let criteria = SearchCriteria::new()
            .in_values("symbol", symbols)
            .gte("date", from_date)
            .sort_asc("date");
        let indicators = self.get_ticker_indicators_by_criteria(&criteria).await?;

        // Group by symbol — sorted order preserved from query
        let mut map: HashMap<String, Vec<TickerIndicator>> = HashMap::new();
        for indicator in indicators {
            map.entry(indicator.symbol.clone())
                .or_default()
                .push(indicator);
        }
        Ok(map)
    }

    async fn get_ticker_indicators_window(&self, symbol: &str) -> Result<IndicatorWindow> {
        let indicators = self.get_ticker_indicators_last_n(symbol, 2).await?;
        let prev = IndicatorSnapshot::from(
            indicators
                .get(1)
                .ok_or(anyhow::anyhow!("Not enough indicators"))?,
        );
        let curr = IndicatorSnapshot::from(
            indicators
                .first()
                .ok_or(anyhow::anyhow!("Not enough indicators"))?,
        );
        let window = IndicatorWindow::new(curr, prev);
        Ok(window)
    }

    async fn save_ticker_indicators(
        &self,
        symbol: &str,
        indicators: Vec<TickerIndicator>,
    ) -> Result<()> {
        match self.manager.ticker_indicators().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.bulk_update(indicators).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!(format!(
                    "Error saving TickerIndicators for {}: {}",
                    symbol, e
                )));
            }
        }
    }
}
