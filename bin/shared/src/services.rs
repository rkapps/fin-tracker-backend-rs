use std::{env, sync::Arc};

use agentic_core::{
    agent::{
        config::{AgentServiceConfig, LocalEndpoint},
        service::AgentService,
    },
    client::embeddings::EmbeddingClient,
    providers::openai::embeddings::OpenAIEmbeddingClient,
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

pub fn get_embedding_client() -> Result<Arc<dyn EmbeddingClient>> {
    let openai_api_key: String =
        env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");
    Ok(Arc::new(OpenAIEmbeddingClient::new(&openai_api_key)?))
}

// get_agent_service
fn get_agent_service() -> Result<AgentService> {
    let openai_api_key: String =
        env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");
    let gemini_api_key: String =
        env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable not set");
    let anthropic_api_key: String =
        env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY environment variable not set");

    // Build config from user credentials + infrastructure config
    let config = AgentServiceConfig::new()
        .with_api_key("openai", &openai_api_key)
        .with_api_key("anthropic", &anthropic_api_key)
        .with_api_key("gemini", &gemini_api_key)
        // .with_local_endpoint(LocalEndpoint {
        //     id: "local".to_string(),
        //     label: "Local".to_string(),
        //     base_url: std::env::var("LOCAL_LLM_BASE_URL")
        //         .expect("LOCAL_LLM_BASE_URL environment variable not found"),
        //     default_model: "qwen3.5:4b".to_string(),
        //     models: vec!["qwen3.5:4b".to_string()],
        // })
        .with_local_endpoint(LocalEndpoint {
            id: "gwen".to_string(),
            label: "Gwen".to_string(),
            base_url: std::env::var("GCP_LLM_BASE_URL").expect("GCP_LLM_BASE_URL environment variable not set"),
            default_model: "qwen3.5:4b".to_string(),
            models: vec!["qwen3.5:4b".to_string()],
        })
        ;

    let agent_service = AgentService::with_config(config);
    Ok(agent_service)
}

pub async fn get_analyse_service() -> Result<AnalyseService> {
    let agent_service = get_agent_service()?;
    let storage_service: Arc<dyn StorageService> = get_storage_service().await?;
    let embedding_client = get_embedding_client()?;
    Ok(AnalyseService::new(
        storage_service,
        embedding_client,
        Arc::new(agent_service),
    ))
}

// Returns the ticker service
pub async fn get_tickers_service() -> Result<TickersService> {
    let storage_service: Arc<dyn StorageService> = get_storage_service().await?;
    Ok(TickersService::new(Arc::clone(&storage_service)))
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
    let mongo_uri = env::var("MONGO_ATLAS_CONN_STR")
        .expect("MONGO_ATLAS_CONN_STR envrionment variable not set");
    println!("MongoAtlas Uri: {}", mongo_uri);

    MongoStorageManager::new(&mongo_uri, "finTracker").await
}
