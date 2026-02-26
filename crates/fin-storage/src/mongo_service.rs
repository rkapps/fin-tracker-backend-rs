use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fin_domain::ticker::{
    Ticker, TickerControl, TickerEmbedding, TickerHistory, TickerIndicator, TickerSentiment,
};
use fin_domain::utils::data_utils::{market_cap_label_range, market_cap_range};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use storage_core::core::Repository;
use storage_core::core::search::{SearchCriteria, SearchOp, SearchValue};
use tracing::debug;

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
                return Err(anyhow::anyhow!("Error getting ticker history: {}", e));
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
                return Err(anyhow::anyhow!("Error getting ticker history: {}", e));
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
                return Err(anyhow::anyhow!("Error getting ticker control: {}", e));
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
                return Err(anyhow::anyhow!("Error getting ticker: {}", e));
            }
        }
    }

    async fn get_tickers(&self) -> Result<Vec<Ticker>> {
        match self.manager.tickers().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.find_all().await
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error getting ticker history: {}", e));
            }
        }
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

        // debug!("SearchCriteria: {:?}", criteria);

        self.get_ticker_by_criteria(&criteria).await
    }

    async fn search_tickers(
        &self,
        industry: Option<String>,
        market_cap_range: Option<String>, // "mega", "large", "mid", "small"
        asset_type: Option<String>,
        signals: Option<Vec<String>>,
    ) -> Result<Vec<Ticker>> {
        let mut criteria = SearchCriteria::new();
        if let Some(industry) = industry {
            criteria.add_condition("industry", SearchOp::Eq, SearchValue::String(industry));
        }

        let new_asset_type = asset_type
            .unwrap_or_else(|| "stock".to_string())
            .to_uppercase();
        criteria.add_condition(
            "asset_type",
            SearchOp::Eq,
            SearchValue::String(new_asset_type),
        );
        if let Some(range) = market_cap_range {
            let (min_cap, max_cap) = market_cap_label_range(Some(range));
            criteria.add_condition("market_cap", SearchOp::Gte, SearchValue::Int(min_cap));
            criteria.add_condition("market_cap", SearchOp::Lte, SearchValue::Int(max_cap));
        }
        if let Some(signals) = signals {
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

    async fn get_ticker_indicators_latest(&self, symbol: &str) -> Result<Vec<TickerIndicator>> {
        let hist = self.get_ticker_history_latest(symbol).await?;
        debug!("hist: {}", hist.len());
        if hist.len() == 0 {
            return Err(anyhow::anyhow!(
                "Error getting ticker indicator. History is empty"
            ));
        }
        let latest_hist = &hist[0];
        let mut criteria = SearchCriteria::new();
        criteria.add_condition(
            "metadata.symbol",
            SearchOp::Eq,
            SearchValue::String(symbol.to_uppercase().to_string()),
        );
        criteria.add_condition(
            "date",
            SearchOp::Gte,
            SearchValue::DateTime(latest_hist.date),
        );
        criteria.add_sort("date", false);

        match self.manager.ticker_indicators().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.find(Some(criteria)).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error getting ticker indicators: {}", e));
            }
        }
    }

    async fn get_ticker_indicators_last_two(&self, symbol: &str) -> Result<Vec<TickerIndicator>> {
        match self.manager.ticker_indicators().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;

                let mut criteria = SearchCriteria::new();
                criteria.add_condition(
                    "metadata.symbol",
                    SearchOp::Eq,
                    SearchValue::String(symbol.to_uppercase().to_string()),
                );
                criteria.add_sort("date", false);
                criteria.add_limit(2);
                let indicators = repo.find(Some(criteria)).await?;
                Ok(indicators)
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error getting ticker indicators: {}", e));
            }
        }
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
                return Err(anyhow::anyhow!("Error getting ticker sentiment: {}", e));
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
                return Err(anyhow::anyhow!("Error getting ticker embedding: {}", e));
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
                debug!("before repo update");
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
}
