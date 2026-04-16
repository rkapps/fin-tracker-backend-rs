use async_trait::async_trait;
use fin_domain::tickers::TickerAlpha;
use storage_core::core::{
    Repository as _,
    search::{SearchCriteria, SearchOp, SearchValue},
};
use tracing::{info, warn};

use crate::{mongo::MongoStorageService, service::TickerAlphaStorageService};
use anyhow::Result;

#[async_trait]
impl TickerAlphaStorageService for MongoStorageService {
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

    async fn save_ticker_alphas(&self, sas: &[TickerAlpha]) -> Result<()> {
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
