use std::{env, sync::Arc};

use anyhow::Result;
use axum::{
    Router,
    routing::{get, post},
};

use bin_shared::services::{get_embedding_client, get_storage_service, get_tickers_service};
use fin_analyse::tools::{
    TickerIndicatorTool, TickerPeersTool, TickerPriceHistoryTool, TickerScreeningTool, TickerSentimentTool, TickerSnapshotTool, TickerTaxonomyTool
};
use fin_services::analyse::AnalyseService;
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
use rustic_agent::Tool;
use rustic_boot::{boot, routes::{conversation::conversation_routes, providers::provider_routes}};
use rustic_core::set_logger;
use tracing::debug;

#[tokio::main]

async fn main() -> Result<()> {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| {
        "agentic_boot=debug,fin_services=debug,fin_providers=info,fin_core=info,agentic_core=info,fin_tracker_pipeline=info,fin_tracker_admin=info,fin_tracker_api=info".to_string()
    });
    set_logger(filter);

    let config_dir = env::var("FINTRACKER_CONFIG_PATH")
        .expect("FINTRACKER_CONFIG_PATH envrionment variable not set");
    let firebase_project_id = env::var("FINTRACKER_PROJECT_ID")
        .expect("FINTRACKER_PROJECT_ID envrionment variable not set");
    // /media/raghu/data2/Workspace/Projects/libs/agentic-boot/data";
    let mongo_db =
        env::var("FINTRACKER_DB_NAME").expect("FINTRACKER_DB_NAME envrionment variable not set");
    let mongo_uri = env::var("MONGO_URI").expect("MONGO_URI envrionment variable not set");
    debug!("Mongo uri: {:?} db: {:?}", mongo_uri, mongo_db);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let origins = [
        "http://localhost:4200",
        "http://localhost:4201",
        "http://localhost:4202",
        "https://fin-tracker-rkapps.web.app",
        "https://rustic-ai-rkapps.web.app",
    ];

    let embedding_client = get_embedding_client()?;
    let storage_service = get_storage_service().await?;
    let ticker_service = get_tickers_service().await?;

    let tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(TickerScreeningTool::new(
            storage_service.clone(),
            embedding_client.clone(),
        )),
        Arc::new(TickerTaxonomyTool::new(storage_service.clone())),
        Arc::new(TickerSentimentTool::new(
            embedding_client.clone(),
            storage_service.clone(),
        )),
        Arc::new(TickerSnapshotTool::new(storage_service.clone())),
        Arc::new(TickerPriceHistoryTool::new(storage_service.clone())),
        Arc::new(TickerIndicatorTool::new(storage_service.clone())),
        Arc::new(TickerPeersTool::new(storage_service.clone())),
    ];

    let fintracker_routes = Router::new()
        .route("/tickers/groups", get(get_ticker_groups_handler))
        .route("/tickers/{symbol}/charts", get(get_ticker_charts_handler))
        .route("/tickers/{symbol}/news", get(get_ticker_news_handler))
        .route("/tickers/search", post(search_tickers_handler))
        // .route("/tickers/analyse", post(analyse_tickers_handler))
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
        .tools(tools)
        .serve(
            &addr,
            |boot| {
                let boot = Arc::new(boot);

                AppState {
                    boot_state: boot.clone(),
                    ticker_service: Arc::new(ticker_service),
                    analyse_service: Arc::new(AnalyseService::new(boot.agent_service.clone())),
                }
            },
            |router, _| router.merge(provider_routes()).merge(fintracker_routes),
            |router, state| router.merge(conversation_routes(state.clone())), // .nest("/finance", finance_routes(state)),
        )
        .await?;

    Ok(())
}
