use anyhow::Result;
use fin_core::tickers::update::{
    update_all_ticker_overview_embeddings, update_all_tickers, update_ticker_overview_embedding,
};
use fin_domain::tickers::{Ticker, TickerControl, TickerSeed};
use fin_providers::ProviderService;
use fin_storage::service::StorageService;
use rustic_ml::EmbeddingClient;
use std::{collections::HashMap, sync::Arc};
use tracing::info;

#[derive(Debug)]
pub struct LoadService {
    pub storage_service: Arc<dyn StorageService>,
    pub provider_service: ProviderService,
    embedding_client: Arc<dyn EmbeddingClient>,
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

    pub async fn load_tickers(&self, ticker_seeds: &[TickerSeed], update: bool) -> Result<()> {
        info!("Loading tickers: {} update: {}", ticker_seeds.len(), update);

        let all_controls = self.storage_service.get_ticker_controls().await?;
        let mut control_map: HashMap<String, TickerControl> = all_controls
            .into_iter()
            .map(|c| (c.symbol.clone(), c))
            .collect();

        let mut all_tickers = Vec::new();
        let mut all_new_controls = Vec::new();
        for seed in ticker_seeds.iter() {
            let tc = control_map
                .remove(&seed.symbol)
                .unwrap_or_else(|| TickerControl::new(seed.clone()));
            let ticker = Ticker::new(seed.clone());
            all_tickers.push(ticker);
            all_new_controls.push(tc);
        }

        let updated_tickers = update_all_tickers(
            self.storage_service.clone(),
            self.provider_service.clone(),
            self.embedding_client.clone(),
            all_new_controls,
            all_tickers.clone(),
            update,
        )
        .await?;

        // update ticker overview embeddings
        update_all_ticker_overview_embeddings(
            self.storage_service.clone(),
            self.embedding_client.clone(),
            all_tickers,
        )
        .await?;

        Ok(())
    }
}
