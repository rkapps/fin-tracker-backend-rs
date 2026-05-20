use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fin_domain::tickers::TickerEmbedding;
use rustic_storage::core::{repository::Repository, search::{SearchCriteria, SearchOp, SearchValue}};
use tracing::debug;

use crate::{mongo::MongoStorageService, service::TickerEmbeddingStorageService};
use anyhow::Result;

#[async_trait]
impl TickerEmbeddingStorageService for MongoStorageService {
    async fn delete_ticker_embeddings_before(&self, date: DateTime<Utc>) -> Result<()> {
        let mut criteria = SearchCriteria::new().lt("date", date);
        // criteria.add_condition("date", SearchOp::Lt, SearchValue::DateTime(date));

        match self.manager.ticker_embeddings().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.delete_many(Some(criteria)).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error getting TickerSentiment: {}", e));
            }
        }
    }

    async fn get_ticker_embeddings(&self, symbol: &str) -> Result<Vec<TickerEmbedding>> {
        let criteria = SearchCriteria::new().eq("symbol", symbol.to_uppercase());
        // criteria.add_condition(
        //     "symbol",
        //     SearchOp::Eq,
        //     SearchValue::String(symbol.to_uppercase().to_string()),
        // );
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

    async fn save_ticker_embeddings(
        &self,
        symbol: &str,
        embeddings: Vec<TickerEmbedding>,
    ) -> Result<()> {
        match self.manager.ticker_embeddings().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.bulk_update(embeddings).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!(format!(
                    "Error saving TickerEmbeddings for {}: {}",
                    symbol, e
                )));
            }
        }
    }
}
