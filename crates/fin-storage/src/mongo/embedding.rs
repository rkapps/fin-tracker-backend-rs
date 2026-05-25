use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fin_domain::tickers::TickerEmbedding;
use rustic_storage::core::{repository::Repository, search::SearchCriteria};

use crate::{mongo::MongoStorageService, service::TickerEmbeddingStorageService};
use anyhow::Result;

#[async_trait]
impl TickerEmbeddingStorageService for MongoStorageService {
    async fn delete_ticker_embeddings_before(&self, date: DateTime<Utc>) -> Result<()> {
        let criteria = SearchCriteria::new().lt("date", date);
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

    async fn get_ticker_embeddings(&self, symbols: Vec<String>) -> Result<Vec<TickerEmbedding>> {
        let criteria = SearchCriteria::new().in_values("symbol", symbols);
        match self.manager.ticker_embeddings().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
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
