use crate::{mongo::MongoStorageService, service::TickerHistoryStorageService};
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fin_domain::tickers::TickerHistory;
use rustic_storage::core::{
    repository::Repository,
    search::SearchCriteria,
};

#[async_trait]
impl TickerHistoryStorageService for MongoStorageService {
    async fn delete_ticker_history(&self, symbol: &str) -> Result<()> {
        let criteria = SearchCriteria::new().eq("metadata.symbol", symbol.to_uppercase());
        // criteria.add_condition(
        //     "metadata.symbol",
        //     SearchOp::Eq,
        //     SearchValue::String(symbol.to_uppercase().to_string()),
        // );
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
        let criteria = SearchCriteria::new().eq("metadata.symbol", symbol.to_uppercase());
        self.get_ticker_history_by_criteria(&criteria).await
    }

    async fn get_ticker_history_by_date(
        &self,
        symbol: &str,
        from_date: DateTime<Utc>,
    ) -> Result<Vec<TickerHistory>> {
        let criteria = SearchCriteria::new()
            .eq("metadata.symbol", symbol.to_uppercase())
            .gte("date", from_date);
        // criteria.add_condition(
        //     "metadata.symbol",
        //     SearchOp::Eq,
        //     SearchValue::String(symbol.to_uppercase().to_string()),
        // );
        // criteria.add_condition("date", SearchOp::Gte, SearchValue::DateTime(from_date));
        self.get_ticker_history_by_criteria(&criteria).await
    }

    async fn get_ticker_history_latest(&self, symbol: &str) -> Result<Vec<TickerHistory>> {
        let criteria = SearchCriteria::new()
            .eq("metadata.symbol", symbol.to_uppercase())
            .sort_desc("date")
            .limit(1);

        // let mut criteria = SearchCriteria::new();
        // criteria.add_condition(
        //     "metadata.symbol",
        //     SearchOp::Eq,
        //     SearchValue::String(symbol.to_uppercase().to_string()),
        // );
        // criteria.add_sort("date", false);
        // criteria.add_limit(1);
        self.get_ticker_history_by_criteria(&criteria).await
    }

    async fn save_ticker_history(&self, symbol: &str, hist: Vec<TickerHistory>) -> Result<()> {
        match self.manager.ticker_history().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.insert_many(hist).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!(format!(
                    "Error saving TickerHistory for {}: {}",
                    symbol, e
                )));
            }
        }
    }
}
