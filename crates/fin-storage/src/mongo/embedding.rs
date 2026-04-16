use async_trait::async_trait;
use fin_domain::tickers::TickerEmbedding;
use storage_core::core::{
    Repository as _,
    search::{SearchCriteria, SearchOp, SearchValue},
};
use tracing::debug;

use crate::{mongo::MongoStorageService, service::TickerEmbeddingStorageService};
use anyhow::Result;

#[async_trait]
impl TickerEmbeddingStorageService for MongoStorageService {
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

    async fn save_ticker_embeddings(
        &self,
        symbol: &str,
        sentiments: &[TickerEmbedding],
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
