use std::{env, sync::Arc};
use anyhow::Result;
use agentic_core::providers::openai::embeddings::OpenAIEmbeddingClient;
use fin_services::{ml::service::MlService, stocks::StocksService, ticker_service::TickerService};
use fin_storage::{
    mongo::{MongoStorageManager, MongoStorageService},
    service::StorageService,
};
use fin_tracker_cli::ticker::fix_ticker_history;
use tracing::{Level, trace};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<()>{
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .with_line_number(true)
        .compact()
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let alpha_key =
        env::var("ALPHA_API_KEY").expect("ALPHA_API_KEY not found in environment variables.");
    let tiingo_token =
        env::var("TIINGO_API_TOKEN").expect("TIINGO_API_TOKEN not found in environment variables.");
    let openai_api_key: String =
        env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    let provider_service = fin_providers::ProviderService::new(&alpha_key, &tiingo_token)?;
    let embedding_client = Arc::new(OpenAIEmbeddingClient::new(&openai_api_key)?);

    let mongo_uri = env::var("MONGO_ATLAS_CONN_STR")
        .expect("MONGO_ATLAS_CONN_STR envrionment variable not set");
    println!("MongoAtlas Uri: {}", mongo_uri);
    let storage_manager = MongoStorageManager::new(&mongo_uri, "test").await?;

    // 2. Wrap in storage service (business logic layer)
    let storage_service: Arc<dyn StorageService> =
        Arc::new(MongoStorageService::new(storage_manager));

    let ml_service = MlService::new(Arc::clone(&storage_service));
    let ticker_service = TickerService::new(Arc::clone(&storage_service));

    let stocks_service = Arc::new(StocksService::new(
        Arc::clone(&storage_service),
        provider_service,
        // Arc::new(agent_service),
        embedding_client.clone(),
        ml_service.clone(),
    ));

    fix_ticker_history(stocks_service.clone()).await?;
    Ok(())
}
