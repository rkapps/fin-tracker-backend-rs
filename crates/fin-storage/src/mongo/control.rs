use async_trait::async_trait;
use fin_domain::tickers::TickerControl;
use storage_core::core::Repository as _;

use crate::{mongo::MongoStorageService, service::TickerControlStorageService};
use anyhow::Result;

#[async_trait]
impl TickerControlStorageService for MongoStorageService {
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
}
