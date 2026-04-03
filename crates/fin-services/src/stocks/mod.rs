use std::sync::Arc;

use agentic_core::client::embeddings::EmbeddingClient;
use fin_providers::ProviderService;
use fin_storage::service::StorageService;

use crate::ml::service::MlService;

// pub mod analyse;
pub mod eod;
pub mod indicators;
pub mod load;
pub mod screen;
pub mod signals;
pub mod update;

const BASE_CURRENCY: &str = "USD";

#[derive(Debug)]
pub struct StocksService {
    pub storage_service: Arc<dyn StorageService>,
    provider_service: ProviderService,
    // agent_service: Arc<AgentService>,
    embedding_client: Arc<dyn EmbeddingClient>,
    ml_service: MlService,
}

impl StocksService {
    pub fn new(
        storage_service: Arc<dyn StorageService>,
        provider_service: ProviderService,
        // agent_service: Arc<AgentService>,
        embedding_client: Arc<dyn EmbeddingClient>,
        ml_service: MlService,
    ) -> StocksService {
        StocksService {
            storage_service,
            provider_service,
            // agent_service,
            embedding_client,
            ml_service,
        }
    }
}
