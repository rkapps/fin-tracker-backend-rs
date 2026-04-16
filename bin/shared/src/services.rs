use std::{env, sync::Arc};

use agentic_core::{
    agent::service::AgentService, providers::openai::embeddings::OpenAIEmbeddingClient,
};
use anyhow::Result;
use fin_providers::ProviderService;
use fin_services::{
    analyse::AnalyseService, load::LoadService, ml::MlService, pipeline::PipeLineService,
    ticker::TickersService,
};
use fin_storage::{
    mongo::{MongoStorageManager, MongoStorageService},
    service::StorageService,
};

// Returns the ticker service
pub async fn get_ticker_service() -> Result<TickersService> {
    let storage_service: Arc<dyn StorageService> = get_storage_service().await?;
    Ok(TickersService::new(Arc::clone(&storage_service)))
}

pub async fn get_analyse_service() -> Result<AnalyseService> {
    let openai_api_key: String =
        env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");
    let gemini_api_key: String =
        env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable not set");
    let anthropic_api_key: String =
        env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY environment variable not set");

    let agent_service = AgentService::new();
    let embedding_client = Arc::new(OpenAIEmbeddingClient::new(&openai_api_key)?);

    let storage_service: Arc<dyn StorageService> = get_storage_service().await?;

    Ok(AnalyseService::new(
        storage_service,
        embedding_client,
        Arc::new(agent_service),
        openai_api_key,
        gemini_api_key,
        anthropic_api_key,
    ))
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
    let openai_api_key: String =
        env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");
    let embedding_client = Arc::new(OpenAIEmbeddingClient::new(&openai_api_key)?);

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
    let openai_api_key: String =
        env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    let embedding_client = Arc::new(OpenAIEmbeddingClient::new(&openai_api_key)?);

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
    fin_providers::ProviderService::new(&alpha_key, &tiingo_token)
}

// Returns the storage service
pub async fn get_storage_service() -> Result<Arc<dyn StorageService>> {
    let storage_manager = get_mongo_manager().await?;
    Ok(Arc::new(MongoStorageService::new(storage_manager)))
}

// Returns the storage manager
async fn get_mongo_manager() -> Result<MongoStorageManager> {
    let mongo_uri = env::var("MONGO_ATLAS_CONN_STR")
        .expect("MONGO_ATLAS_CONN_STR envrionment variable not set");
    println!("MongoAtlas Uri: {}", mongo_uri);

    MongoStorageManager::new(&mongo_uri, "finTracker").await
}
