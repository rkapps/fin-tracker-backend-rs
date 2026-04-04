use std::collections::HashMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fin_domain::ticker::{IndicatorSnapshot, IndicatorWindow, TickerIndicator};
use storage_core::core::{
    Repository as _,
    search::{SearchCriteria, SearchOp, SearchValue},
};
use tracing::debug;

use crate::{
    mongo::MongoStorageService,
    service::{TickerIndicatorStorageService, TickerStorageService},
};
use anyhow::Result;

#[async_trait]
impl TickerIndicatorStorageService for MongoStorageService {

    async fn delete_ticker_indicators(&self, symbol: &str) -> Result<()> {
        let mut criteria = SearchCriteria::new();
        criteria.add_condition(
            "symbol",
            SearchOp::Eq,
            SearchValue::String(symbol.to_uppercase().to_string()),
        );
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

    async fn get_ticker_indicators(&self, symbol: &str) -> Result<Vec<TickerIndicator>> {
        let mut criteria = SearchCriteria::new();
        criteria.add_condition(
            "symbol",
            SearchOp::Eq,
            SearchValue::String(symbol.to_uppercase().to_string()),
        );
        criteria.add_sort("date", true);
        self.get_ticker_indicators_by_criteria(&criteria).await
    }

    async fn get_ticker_indicators_by_symbol(
        &self,
        symbol: &str,
        from_date: DateTime<Utc>,
    ) -> Result<Vec<TickerIndicator>> {
        let mut criteria = SearchCriteria::new();
        criteria.add_condition(
            "symbol",
            SearchOp::Eq,
            SearchValue::String(symbol.to_uppercase().to_string()),
        );
        criteria.add_condition("date", SearchOp::Gte, SearchValue::DateTime(from_date));
        criteria.add_sort("date", true);
        self.get_ticker_indicators_by_criteria(&criteria).await
    }

    async fn get_ticker_indicators_latest(&self, symbol: &str) -> Result<TickerIndicator> {
        let indicators = self.get_ticker_indicators_last_n(&symbol, 1).await?;
        let indicator = indicators.get(0).unwrap();
        Ok(indicator.clone())
    }

    async fn get_ticker_indicators_last_n(
        &self,
        symbol: &str,
        n: usize,
    ) -> Result<Vec<TickerIndicator>> {
        let mut criteria = SearchCriteria::new();
        criteria.add_condition(
            "symbol",
            SearchOp::Eq,
            SearchValue::String(symbol.to_uppercase().to_string()),
        );
        criteria.add_sort("date", false);
        criteria.add_limit(n);
        self.get_ticker_indicators_by_criteria(&criteria).await
    }

    async fn get_ticker_indicators_map_by_sector(
        &self,
        sector: &str,
        from_date: DateTime<Utc>,
    ) -> Result<HashMap<String, Vec<TickerIndicator>>> {
        let tickers = self.get_ticker_by_sector(&sector).await?;
        let symbols: Vec<String> = tickers.iter().map(|t| t.symbol.clone()).collect();

        debug!("Tickers for sector: {}-{:?}", sector, symbols);
        let mut criteria = SearchCriteria::new();
        criteria.add_condition("symbol", SearchOp::In, SearchValue::Array(symbols));
        criteria.add_condition("date", SearchOp::Gte, SearchValue::DateTime(from_date));

        criteria.add_sort("date", true);
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
                .get(0)
                .ok_or(anyhow::anyhow!("Not enough indicators"))?,
        );
        let window = IndicatorWindow::new(curr, prev);
        Ok(window)
    }

    async fn save_ticker_indicators(
        &self,
        symbol: &str,
        indicators: &Vec<TickerIndicator>,
    ) -> Result<()> {
        let Ok(repo) = self.manager.ticker_indicators().await else {
            return Err(anyhow::anyhow!(format!(
                "Error saving TickerIndicator for '{}'",
                symbol
            )));
        };
        let mut repo = repo.lock().await;
        for indicator in indicators {
            repo.insert(indicator.clone()).await?;
        }
        Ok(())
    }
}
