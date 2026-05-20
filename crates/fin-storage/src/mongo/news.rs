use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fin_domain::tickers::TickerNews;
use rustic_storage::core::{repository::Repository, search::{SearchCriteria, SearchOp, SearchValue}};
use tracing::debug;

use crate::{mongo::MongoStorageService, service::TickerNewsStorageService};
use anyhow::Result;

#[async_trait]
impl TickerNewsStorageService for MongoStorageService {
    async fn delete_ticker_news_before(&self, date: DateTime<Utc>) -> Result<()> {
        let criteria = SearchCriteria::new().lt("date", date);
        // criteria.add_condition("date", SearchOp::Lt, SearchValue::DateTime(date));

        match self.manager.ticker_news().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.delete_many(Some(criteria)).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error getting TickerSentiment: {}", e));
            }
        }
    }

    async fn get_ticker_news(&self, symbol: &str) -> Result<Vec<TickerNews>> {
        let criteria = SearchCriteria::new().eq("symbol", symbol.to_uppercase()).limit(50).sort_desc("date");
        // criteria.add_condition(
        //     "symbol",
        //     SearchOp::Eq,
        //     SearchValue::String(symbol.to_uppercase().to_string()),
        // );
        // criteria.add_limit(50);
        // criteria.add_sort("date", false);
        // debug!("Criteria: {:?}", criteria);
        match self.manager.ticker_news().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                debug!("Criteria: {:?}", criteria);
                repo.find(Some(criteria)).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error getting TickerNews: {}", e));
            }
        }
    }

    async fn save_ticker_news(&self, symbol: &str, embeddings: Vec<TickerNews>) -> Result<()> {
        match self.manager.ticker_news().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.bulk_update(embeddings).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!(format!(
                    "Error saving TickerNewss for {}: {}",
                    symbol, e
                )));
            }
        }
    }
}
