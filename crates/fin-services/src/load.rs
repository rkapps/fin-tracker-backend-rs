use agentic_core::client::embeddings::EmbeddingClient;
use anyhow::Result;
use fin_core::tickers::update::{update_ticker, update_ticker_overview_embedding};
use fin_domain::tickers::{Ticker, TickerControl, TickerSeed};
use fin_providers::ProviderService;
use fin_storage::service::StorageService;
use std::{sync::Arc, time::Duration};
use tokio::time::sleep;
use tracing::{error, info};

#[derive(Debug)]
pub struct LoadService {
    pub storage_service: Arc<dyn StorageService>,
    pub provider_service: ProviderService,
    pub embedding_client: Arc<dyn EmbeddingClient>,
}

impl LoadService {
    pub fn new(
        storage_service: Arc<dyn StorageService>,
        provider_service: ProviderService,
        embedding_client: Arc<dyn EmbeddingClient>,
    ) -> LoadService {
        LoadService {
            storage_service,
            provider_service,
            embedding_client,
        }
    }

    pub async fn load_tickers(&self, ticker_seeds: &[TickerSeed]) -> Result<()> {
        info!("Loading tickers: {}", ticker_seeds.len());

        let delay = Duration::from_millis(500); // Sleep for 0 milliseconds

        let mut count = 0;
        let length = ticker_seeds.len();
        for (i, seed) in ticker_seeds.iter().enumerate() {
            if i % 20 == 0 {
                info!("Loading Ticker: {} {}/{}", seed.symbol, i + 1, length);
            }

            let (mut tc, mut ticker) = {
                let tc = self
                    .storage_service
                    .get_ticker_control(&seed.symbol.clone())
                    .await
                    .unwrap_or_else(|_| TickerControl::new(seed.clone()));

                let ticker = Ticker::new(seed.clone());
                (tc, ticker)
            };

            let storage_service = self.storage_service.clone();
            let provider_service = self.provider_service.clone();
            let embedding_client = self.embedding_client.clone();

            // Update and save
            if let Err(e) = update_ticker(
                storage_service.clone(),
                provider_service,
                embedding_client.clone(),
                &mut tc,
                &mut ticker,
            )
            .await
            {
                error!("Ticker {}: {}", seed.symbol, e);
                continue;
            }

            if let Err(e) =
                update_ticker_overview_embedding(storage_service, embedding_client, &mut ticker).await
            {
                error!("Ticker overview embedding {}: {}", seed.symbol, e);
                continue;
            } // break;

            sleep(delay).await;
            count += 1;
            // break;
        }

        info!("Loaded {} Tickers.", count);
        Ok(())
    }

}
