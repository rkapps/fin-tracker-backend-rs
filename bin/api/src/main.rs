use anyhow::Result;
use axum::{
    Router,
    routing::{get, post},
};
use std::{env, sync::Arc};

use fin_tracker_api::{
    handlers::{
        analyse::analyse_tickers_streaming_handler,
        tickers::{
            get_ticker_charts_handler, get_ticker_groups_handler, get_ticker_news_handler,
            search_tickers_handler,
        },
    },
    state::AppState,
};
use rustic_boot::{
    boot,
    routes::{conversation::conversation_routes, providers::provider_routes},
};
use rustic_core::{logger::set_logger_with_telemetry, set_logger};
use rustic_finance::service::FinanceService;
use rustic_ml::embeddings::openai::OpenAIEmbeddingClient;
use tracing::debug;

#[tokio::main]

async fn main() -> Result<()> {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| {
        "rustic_boot=info,rustic_agent=debug,fin_services=info,fin_analyse=info,fin_core=info"
            .to_string()
    });

    let config_dir = env::var("FINTRACKER_CONFIG_PATH")
        .expect("FINTRACKER_CONFIG_PATH envrionment variable not set");
    let firebase_project_id = env::var("FINTRACKER_PROJECT_ID")
        .expect("FINTRACKER_PROJECT_ID envrionment variable not set");

    let endpoint = std::env::var("OTEL_ENDPOINT")?;
        set_logger_with_telemetry(filter, "fin-tracker-api", &firebase_project_id, &endpoint).await?;
    
    let mongo_db = env::var("RUSTIC_FINANCE_DB_NAME")
        .expect("RUSTIC_FINANCE_DB_NAME envrionment variable not set");
    
    let mongo_uri = env::var("RUSTIC_FINANCE_MONGO_URI").expect("MONGO_URI envrionment variable not set");
    debug!("Mongo uri: {:?} db: {:?}", mongo_uri, mongo_db);

    let openai_api_key: String =
        env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");
    let embedding_client = Arc::new(OpenAIEmbeddingClient::new(&openai_api_key)?);

    let finance_service =
        FinanceService::new_reader(&mongo_uri, &mongo_db, embedding_client).await?;

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let origins = [
        "http://localhost:4200",
        "http://localhost:4201",
        "http://localhost:4202",
        "https://fin-tracker-rkapps.web.app",
        "https://rustic-ai-rkapps.web.app",
    ];

    let fintracker_routes = Router::new()
        .route("/tickers/groups", get(get_ticker_groups_handler))
        .route("/tickers/{symbol}/charts", get(get_ticker_charts_handler))
        .route("/tickers/{symbol}/news", get(get_ticker_news_handler))
        .route("/tickers/search", post(search_tickers_handler))
        .route(
            "/tickers/analyse_streaming",
            post(analyse_tickers_streaming_handler),
        );

    boot::AgenticBootBuilder::new()
        .config_dir(config_dir.to_string())
        .firebase_project_id(&firebase_project_id)
        .providers("providers.json".to_string())
        .agents_config("agents.json".to_string())
        .mongo_database(mongo_uri, mongo_db)
        .cors_origins(origins.to_vec())
        .tools(finance_service.tools())
        .serve(
            &addr,
            |boot| {
                let boot = Arc::new(boot);

                AppState {
                    boot_state: boot.clone(),
                    finance_service: Arc::new(finance_service),
                }
            },
            |router, _| router.merge(provider_routes()).merge(fintracker_routes),
            |router, state| router.merge(conversation_routes(state.clone())), // .nest("/finance", finance_routes(state)),
        )
        .await?;

    Ok(())
}
