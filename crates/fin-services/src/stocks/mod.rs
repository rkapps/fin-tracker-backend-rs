use std::sync::Arc;

use agentic_core::capabilities::client::embeddings::EmbeddingClient;
use fin_providers::ProviderService;
use fin_storage::service::StorageService;

pub mod analyse;
pub mod eod;
pub mod load;
pub mod update;
pub mod indicators;
pub mod signals;

pub struct StocksService {
    pub storage_service: Arc<dyn StorageService>,
    provider_service: ProviderService,
    // agent_service: Arc<AgentService>,
    embedding_client: Arc<dyn EmbeddingClient>,
}

impl StocksService {
    pub fn new(
        storage_service: Arc<dyn StorageService>,
        provider_service: ProviderService,
        // agent_service: Arc<AgentService>,
        embedding_client: Arc<dyn EmbeddingClient>,
    ) -> StocksService {
        StocksService {
            storage_service,
            provider_service,
            // agent_service,
            embedding_client,
        }
    }
}

