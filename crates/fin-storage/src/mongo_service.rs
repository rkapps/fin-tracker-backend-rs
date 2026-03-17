use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fin_domain::dto::screen_param::TickerScreenParam;
use fin_domain::ticker::{
    IndicatorSnapshot, IndicatorWindow, Ticker, TickerAlpha, TickerControl, TickerEmbedding,
    TickerHistory, TickerIndicator, TickerSentiment,
};
use fin_domain::utils::data_utils::{market_cap_label_range, market_cap_range};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use storage_core::core::Repository;
use storage_core::core::search::{SearchCriteria, SearchOp, SearchValue};
use tracing::{debug, info, warn};

use crate::{mongo_manager::MongoStorageManager, service::StorageService};

#[derive(Debug)]
pub struct MongoStorageService {
    manager: MongoStorageManager,
}
impl MongoStorageService {
    pub fn new(manager: MongoStorageManager) -> Self {
        Self { manager }
    }

    async fn get_ticker_by_criteria(&self, criteria: &SearchCriteria) -> Result<Vec<Ticker>> {
        match self.manager.tickers().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.find(Some(criteria.clone())).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error getting Ticker: {}", e));
            }
        }
    }

    async fn get_ticker_history_by_criteria(
        &self,
        criteria: &SearchCriteria,
    ) -> Result<Vec<TickerHistory>> {
        match self.manager.ticker_history().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.find(Some(criteria.clone())).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error getting TickerHistory: {}", e));
            }
        }
    }

    async fn get_ticker_indicators_by_criteria(
        &self,
        criteria: &SearchCriteria,
    ) -> Result<Vec<TickerIndicator>> {
        match self.manager.ticker_indicators().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.find(Some(criteria.clone())).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error getting TickerIndicator: {}", e));
            }
        }
    }
}

#[async_trait]
impl StorageService for MongoStorageService {
    async fn get_ticker_control(&self, symbol: &str) -> Result<TickerControl> {
        match self.manager.ticker_controls().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.find_by_id(symbol.to_string()).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error getting TickerControl: {}", e));
            }
        }
    }

    async fn get_ticker(&self, symbol: &str) -> Result<Ticker> {
        match self.manager.tickers().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.find_by_id(symbol.to_string()).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error getting Ticker: {}", e));
            }
        }
    }

    async fn get_tickers(&self) -> Result<Vec<Ticker>> {
        let mut criteria = SearchCriteria::new();
        criteria.add_sort("symbol", true);
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_ticker_groups(&self) -> Result<HashMap<String, Vec<String>>> {
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();
        let tickers = self.get_tickers().await?;
        for ticker in tickers {
            if let (Some(sector), Some(industry)) = (ticker.sector, ticker.industry) {
                let industries = groups.entry(sector).or_insert_with(Vec::new);
                if !industries.contains(&industry) {
                    industries.push(industry);
                }
            }
        }
        Ok(groups)
    }

    async fn get_ticker_by_sector(&self, sector: &str) -> Result<Vec<Ticker>> {
        let mut criteria = SearchCriteria::new();
        criteria.add_condition(
            "sector",
            SearchOp::Eq,
            SearchValue::String(sector.to_string()),
        );
        criteria.add_sort("market_cap", false);
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_tickers_by_marketcap(&self) -> Result<Vec<Ticker>> {
        let mut criteria = SearchCriteria::new();
        criteria.add_sort("market_cap", false);
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_ticker_peers_by_industry(&self, symbol: &str) -> Result<Vec<Ticker>> {
        let ticker = self.get_ticker(symbol).await?;
        let industry = ticker.industry.unwrap_or_default();
        if industry.len() == 0 {
            return Ok(Vec::new());
        }
        // Get reference ticker market cap bucket
        let (min_cap, max_cap) = market_cap_range(ticker.market_cap);

        let mut criteria = SearchCriteria::new();
        criteria.add_condition("industry", SearchOp::Eq, SearchValue::String(industry));
        criteria.add_condition("market_cap", SearchOp::Gte, SearchValue::Int(min_cap));
        criteria.add_condition("market_cap", SearchOp::Lte, SearchValue::Int(max_cap));
        criteria.add_sort("market_cap", false);
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_ticker_peers_by_sector(&self, symbol: &str) -> Result<Vec<Ticker>> {
        let ticker = self.get_ticker(symbol).await?;
        let sector = ticker.sector.unwrap_or_default();
        if sector.len() == 0 {
            return Ok(Vec::new());
        }
        // Get reference ticker market cap bucket
        let (min_cap, max_cap) = market_cap_range(ticker.market_cap);

        let mut criteria = SearchCriteria::new();
        criteria.add_condition("sector", SearchOp::Eq, SearchValue::String(sector));
        criteria.add_condition("market_cap", SearchOp::Gte, SearchValue::Int(min_cap));
        criteria.add_condition("market_cap", SearchOp::Lte, SearchValue::Int(max_cap));
        criteria.add_sort("market_cap", false);

        self.get_ticker_by_criteria(&criteria).await
    }

    async fn search_tickers(&self, param: TickerScreenParam) -> Result<Vec<Ticker>> {
        let mut criteria = SearchCriteria::new();
        if let Some(industry) = param.industry {
            criteria.add_condition("industry", SearchOp::Eq, SearchValue::String(industry));
        }

        let new_asset_type = param
            .asset_type
            .unwrap_or_else(|| "stock".to_string())
            .to_uppercase();
        criteria.add_condition(
            "asset_type",
            SearchOp::Eq,
            SearchValue::String(new_asset_type),
        );
        if let Some(range) = param.market_cap_range {
            let (min_cap, max_cap) = market_cap_label_range(Some(range));
            criteria.add_condition("market_cap", SearchOp::Gte, SearchValue::Int(min_cap));
            criteria.add_condition("market_cap", SearchOp::Lte, SearchValue::Int(max_cap));
        }
        if let Some(signals) = param.signals {
            criteria.add_condition("signals", SearchOp::All, SearchValue::Array(signals));
        }
        criteria.add_sort("market_cap", false);

        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_ticker_history(&self, symbol: &str) -> Result<Vec<TickerHistory>> {
        let mut criteria = SearchCriteria::new();
        criteria.add_condition(
            "metadata.symbol",
            SearchOp::Eq,
            SearchValue::String(symbol.to_uppercase().to_string()),
        );
        self.get_ticker_history_by_criteria(&criteria).await
    }

    async fn get_ticker_history_by_date(
        &self,
        symbol: &str,
        from_date: DateTime<Utc>,
    ) -> Result<Vec<TickerHistory>> {
        let mut criteria = SearchCriteria::new();
        criteria.add_condition(
            "metadata.symbol",
            SearchOp::Eq,
            SearchValue::String(symbol.to_uppercase().to_string()),
        );
        criteria.add_condition("date", SearchOp::Gte, SearchValue::DateTime(from_date));
        self.get_ticker_history_by_criteria(&criteria).await
    }

    async fn get_ticker_history_latest(&self, symbol: &str) -> Result<Vec<TickerHistory>> {
        let mut criteria = SearchCriteria::new();
        criteria.add_condition(
            "metadata.symbol",
            SearchOp::Eq,
            SearchValue::String(symbol.to_uppercase().to_string()),
        );
        criteria.add_sort("date", false);
        criteria.add_limit(1);
        self.get_ticker_history_by_criteria(&criteria).await
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

    async fn get_ticker_sentiments(&self, symbol: &str) -> Result<Vec<TickerSentiment>> {
        let score = dec!(0);
        self.get_ticker_sentiments_with_score(symbol, &score).await
    }

    async fn get_ticker_sentiments_with_score(
        &self,
        symbol: &str,
        score: &Decimal,
    ) -> Result<Vec<TickerSentiment>> {
        let mut criteria = SearchCriteria::new();
        criteria.add_condition(
            "symbol",
            SearchOp::Eq,
            SearchValue::String(symbol.to_uppercase().to_string()),
        );
        criteria.add_condition(
            "relevance_score",
            SearchOp::Gte,
            SearchValue::Decimal(score.clone()),
        );

        match self.manager.ticker_sentiments().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.find(Some(criteria)).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error getting TickerSentiment: {}", e));
            }
        }
    }

    async fn get_ticker_embeddings(&self, symbol: &str) -> Result<Vec<TickerEmbedding>> {
        let mut criteria = SearchCriteria::new();
        criteria.add_condition(
            "symbol",
            SearchOp::Eq,
            SearchValue::String(symbol.to_uppercase().to_string()),
        );
        // debug!("Criteria: {:?}", criteria);
        match self.manager.ticker_embeddings().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                debug!("Criteria: {:?}", criteria);
                repo.find(Some(criteria)).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error getting TickerEmbedding: {}", e));
            }
        }
    }

    async fn get_ticker_overview_embeddings(&self) -> Result<Vec<(Ticker, Vec<f32>)>> {
        let tickers = self.get_tickers().await?;
        let candidates = tickers
            .into_iter()
            .filter_map(|t| t.overview_embedding.clone().map(|embedding| (t, embedding)))
            .collect();

        Ok(candidates)
    }

    async fn get_ticker_industry_embeddings(&self) -> Result<Vec<(Ticker, Vec<f32>)>> {
        let tickers = self.get_tickers().await?;
        let candidates = tickers
            .into_iter()
            .filter_map(|t| t.industry_embedding.clone().map(|embedding| (t, embedding)))
            .collect();

        Ok(candidates)
    }

    async fn save_ticker_control(&self, tc: TickerControl) -> Result<()> {
        match self.manager.ticker_controls().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.update(tc).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!(format!(
                    "Error saving TickerControl for '{}' error: {}",
                    tc.symbol, e
                )));
            }
        }
    }

    // save ticker or return error
    async fn save_ticker(&self, ticker: Ticker) -> Result<()> {
        let result = match self.manager.tickers().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.update(ticker).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!(format!(
                    "Error saving Ticker for '{}' error: {}",
                    ticker.symbol, e
                )));
            }
        };
        result
    }

    async fn save_ticker_history(&self, symbol: &str, hist: &Vec<TickerHistory>) -> Result<()> {
        let Ok(repo) = self.manager.ticker_history().await else {
            return Err(anyhow::anyhow!(format!(
                "Error saving TickerHistory for '{}'",
                symbol
            )));
        };

        let mut repo = repo.lock().await;
        for thist in hist {
            repo.insert(thist.clone()).await?;
        }
        Ok(())
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

    async fn save_ticker_sentiments(
        &self,
        symbol: &str,
        sentiments: &Vec<TickerSentiment>,
    ) -> Result<()> {
        let Ok(repo) = self.manager.ticker_sentiments().await else {
            return Err(anyhow::anyhow!(format!(
                "Error saving TickerSentiment for '{}'",
                symbol
            )));
        };
        let mut repo = repo.lock().await;
        for sentiment in sentiments {
            repo.insert(sentiment.clone()).await?;
        }
        Ok(())
    }

    async fn save_ticker_embeddings(
        &self,
        symbol: &str,
        sentiments: &Vec<TickerEmbedding>,
    ) -> Result<()> {
        let Ok(repo) = self.manager.ticker_embeddings().await else {
            return Err(anyhow::anyhow!(format!(
                "Error saving TickerEmbedding for '{}'",
                symbol
            )));
        };
        let mut repo = repo.lock().await;
        for sentiment in sentiments {
            repo.insert(sentiment.clone()).await?;
        }
        Ok(())
    }

    async fn get_ticker_alphas_by_key(&self, key: &str) -> Result<Vec<TickerAlpha>> {
        let Ok(repo) = self.manager.ticker_alphas().await else {
            return Err(anyhow::anyhow!("Error saving TickerAlpha",));
        };
        let mut repo = repo.lock().await;
        let mut criteria = SearchCriteria::new();
        criteria.add_condition("key", SearchOp::Eq, SearchValue::String(key.to_string()));
        criteria.add_sort("date", false);

        // for each ticker/sector there are 4 (periods) x 2 (algorithm lf/rf) =  8 records
        // criteria.add_limit(8);

        repo.find(Some(criteria)).await
    }

    async fn save_ticker_alphas(&self, sas: &Vec<TickerAlpha>) -> Result<()> {
        let Ok(repo) = self.manager.ticker_alphas().await else {
            return Err(anyhow::anyhow!("Error saving SectorAlpha"));
        };
        let mut repo = repo.lock().await;
        let mut saved = 0;
        let mut failed = 0;

        for sa in sas {
            match repo.insert(sa.clone()).await {
                Ok(_) => saved += 1,
                Err(e) => {
                    warn!("Failed to save alpha {}:{} — {}", sa.key, sa.n, e);
                    failed += 1;
                }
            }
        }

        info!("Saved {} alphas, {} failed", saved, failed);
        Ok(())
    }
}
