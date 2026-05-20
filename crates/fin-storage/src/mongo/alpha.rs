use crate::{mongo::MongoStorageService, service::TickerAlphaStorageService};
use anyhow::Result;
use async_trait::async_trait;
use fin_domain::tickers::TickerAlpha;
use rustic_storage::core::{repository::Repository, search::SearchCriteria};

#[async_trait]
impl TickerAlphaStorageService for MongoStorageService {
    async fn get_ticker_alphas_by_key(&self, key: &str) -> Result<Vec<TickerAlpha>> {
        let Ok(repo) = self.manager.ticker_alphas().await else {
            return Err(anyhow::anyhow!("Error saving TickerAlpha",));
        };
        let mut repo = repo.lock().await;
        let criteria = SearchCriteria::new().eq("key", key).sort_desc("date");
        // criteria.add_condition("key", SearchOp::Eq, SearchValue::String(key.to_string()));
        // criteria.add_sort("date", false);

        repo.find(Some(criteria)).await
    }

    async fn save_ticker_alphas(&self, sas: Vec<TickerAlpha>) -> Result<()> {
        match self.manager.ticker_alphas().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.bulk_update(sas).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!(format!("Error saving TickerAlpha: {}", e)));
            }
        }
    }
}
