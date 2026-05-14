use std::{env, sync::Arc};

use agentic_core::{
    client::embeddings::EmbeddingClient, providers::openai::embeddings::OpenAIEmbeddingClient,
};
use anyhow::Result;
use fin_providers::ProviderService;
use fin_services::{
    load::LoadService, ml::MlService, pipeline::PipeLineService, ticker::TickersService,
};
use fin_storage::{
    mongo::{MongoStorageManager, MongoStorageService},
    service::StorageService,
};

pub fn get_embedding_client() -> Result<Arc<dyn EmbeddingClient>> {
    let openai_api_key: String =
        env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");
    Ok(Arc::new(OpenAIEmbeddingClient::new(&openai_api_key)?))
}

// pub async fn get_analyse_service(agent_service: AgentService) -> Result<AnalyseService> {
//     // let agent_service = get_agent_service()?;
//     let storage_service: Arc<dyn StorageService> = get_storage_service().await?;
//     let embedding_client = get_embedding_client()?;
//     Ok(AnalyseService::new(
//         storage_service,
//         embedding_client,
//         Arc::new(agent_service),
//     ))
// }

// Returns the ticker service
pub async fn get_tickers_service() -> Result<TickersService> {
    let storage_service: Arc<dyn StorageService> = get_storage_service().await?;
    let embedding_client = get_embedding_client()?;
    Ok(TickersService::new(Arc::clone(&storage_service), embedding_client))
}

//Returns the Ml service
pub async fn get_ml_service() -> Result<MlService> {
    let storage_service: Arc<dyn StorageService> = get_storage_service().await?;
    Ok(MlService::new(storage_service))
}

// Returns the load service
pub async fn get_load_service() -> Result<LoadService> {
    let storage_service: Arc<dyn StorageService> = get_storage_service().await?;
    let provider_service = get_provider_service()?;
    let embedding_client = get_embedding_client()?;

    Ok(LoadService::new(
        storage_service,
        provider_service,
        embedding_client,
    ))
}

// Returns the pipeline service
pub async fn get_pipeline_service() -> Result<PipeLineService> {
    let storage_service: Arc<dyn StorageService> = get_storage_service().await?;
    let provider_service = get_provider_service()?;
    let embedding_client = get_embedding_client()?;

    Ok(PipeLineService::new(
        storage_service,
        provider_service,
        embedding_client,
    ))
}

// Returns the provider service
pub fn get_provider_service() -> Result<ProviderService> {
    let alpha_key =
        env::var("ALPHA_API_KEY").expect("ALPHA_API_KEY not found in environment variables.");
    let tiingo_token =
        env::var("TIINGO_API_TOKEN").expect("TIINGO_API_TOKEN not found in environment variables.");
    let coinmarketcap_key = env::var("COINMARKETCAP_API_KEY")
        .expect("COINMARKETCAP_API_KEY not found in environment variables.");

    fin_providers::ProviderService::new(&alpha_key, &tiingo_token, &coinmarketcap_key)
}

// Returns the storage service
pub async fn get_storage_service() -> Result<Arc<dyn StorageService>> {
    let storage_manager = get_mongo_manager().await?;
    Ok(Arc::new(MongoStorageService::new(storage_manager)))
}

// Returns the storage manager
async fn get_mongo_manager() -> Result<MongoStorageManager> {
    let mongo_db =
        env::var("FINTRACKER_DB_NAME").expect("FINTRACKER_DB_NAME envrionment variable not set");
    let mongo_uri = env::var("MONGO_URI").expect("MONGO_URI envrionment variable not set");

    MongoStorageManager::new(&mongo_uri, &mongo_db).await
}
