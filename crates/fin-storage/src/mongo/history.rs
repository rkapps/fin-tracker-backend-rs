use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fin_domain::tickers::TickerHistory;
use storage_core::core::{
    Repository as _,
    search::{SearchCriteria, SearchOp, SearchValue},
};

use crate::{mongo::MongoStorageService, service::TickerHistoryStorageService};
use anyhow::Result;

#[async_trait]
impl TickerHistoryStorageService for MongoStorageService {
    async fn delete_ticker_history(&self, symbol: &str) -> Result<()> {
        let mut criteria = SearchCriteria::new();
        criteria.add_condition(
            "metadata.symbol",
            SearchOp::Eq,
            SearchValue::String(symbol.to_uppercase().to_string()),
        );
        let Ok(repo) = self.manager.ticker_history().await else {
            return Err(anyhow::anyhow!(format!(
                "Error finding TickerHistory for '{}'",
                symbol
            )));
        };

        let mut repo = repo.lock().await;
        repo.delete_many(Some(criteria)).await?;

        Ok(())
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

    async fn save_ticker_history(&self, symbol: &str, hist: Vec<TickerHistory>) -> Result<()> {

        match self.manager.ticker_history().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.bulk_update(hist).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!(format!("Error saving Ticker: {}", e)));
            }
        }

        // let Ok(repo) = self.manager.ticker_history().await else {
        //     return Err(anyhow::anyhow!(format!(
        //         "Error saving TickerHistory for '{}'",
        //         symbol
        //     )));
        // };

        // let mut repo = repo.lock().await;
        // for thist in hist {
        //     repo.insert(thist.clone()).await?;
        // }
    }
}
