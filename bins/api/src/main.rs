use std::{env, sync::Arc};

use agentic_core::agent::service::AgentService;
use agentic_core::providers::openai::embeddings::OpenAIEmbeddingClient;
use anyhow::Result;
use axum::{
    Router,
    http::HeaderValue,
    routing::{get, post},
};

use fin_http::HttpClient;
use fin_services::{stocks::StocksService, tools::ToolsService};
use fin_storage::{
    mongo_manager::MongoStorageManager, mongo_service::MongoStorageService, service::StorageService,
};
use fin_tracker_api::{
    handlers::{
        self,
        stocks::{
            get_ticker_embeddings, get_ticker_sentiments, get_tickers,
        }, tools::analyse_tickers_handler,
    },
    middleware,
    state::AppState,
};
use fin_tracker_api::{
    handlers::{
        cron::{handle_ticker_embeddings_eod, handle_ticker_predictions_eod, handle_tickers_eod},
        stocks::{
            get_ticker_history, get_ticker_history_latest, get_ticker_indicators_latest,
            screen_tickers_handler, search_tickers_handler,
        },
    },
};

use reqwest::Method;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;
use tracing::{Level, debug};
use tracing_subscriber::{filter, fmt, prelude::*};

#[tokio::main]

async fn main() -> Result<()> {
    let filter = filter::Targets::new()
        .with_target("storage_core::mongo", Level::INFO)
        // .with_target("storage_core::vector", Level::DEBUG)
        .with_target("agentic_core::http", Level::INFO)
        .with_target("agentic_core::agent", Level::INFO)
        .with_target("agentic_core::providers", Level::INFO)
        // .with_target("fin_tracker_backend_rs::http", Level::DEBUG)
        .with_target("fin_tracker_api", Level::DEBUG)
        .with_target("fin_services", Level::INFO)
        // .with_target("fin_storage", Level::INFO)
        .with_target("fin_providers", Level::INFO);
    tracing_subscriber::registry()
        .with(
            fmt::layer().event_format(
                fmt::format()
                    // .with_file(false)
                    // .with_line_number(true)
                    .compact(), // .pretty(),
            ),
        ) // Compact format
        .with(filter)
        .init();

    // StocksService (async)
    // → locks storage: storage.lock().await
    //     → StocksStorage (async methods)
    //         → FsDatabase (async methods)
    //             → FileRepository (async methods)
    //                 → File I/O (sync operations)

    // let finance_db_path =
    //     env::var("FINANCE_DB_PATH").expect("FINANCE_DB_PATH not found in environment variables.");
    // let finance_db_name =
    //     env::var("FINANCE_DB_NAME").expect("FINANCE_DB_NAME not found in environment variables.");
    let http_client = HttpClient::new().expect("Http Client cannot be configured.");
    let alpha_key =
        env::var("ALPHA_API_KEY").expect("ALPHA_API_KEY not found in environment variables.");
    let tiingo_token =
        env::var("TIINGO_API_TOKEN").expect("TIINGO_API_TOKEN not found in environment variables.");
    let openai_api_key: String =
        env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");
    let gemini_api_key: String =
        env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable not set");
    let anthropic_api_key: String =
        env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY environment variable not set");

    // stocks
    let embedding_client = Arc::new(OpenAIEmbeddingClient::new(&openai_api_key)?);
    let provider_service =
        fin_providers::ProviderService::new(http_client, &alpha_key, &tiingo_token);
    // let stocks_storage = FileStorageManager::new(finance_db_name, finance_db_path).await?;

    let mongo_uri = env::var("MONGO_ATLAS_CONN_STR")
        .expect("MONGO_ATLAS_CONN_STR envrionment variable not set");
    debug!("MongoAtlas Uri: {}", mongo_uri);
    let storage_manager = MongoStorageManager::new(&mongo_uri, "test").await?;

    // 2. Wrap in storage service (business logic layer)
    let storage_service: Arc<dyn StorageService> =
        Arc::new(MongoStorageService::new(storage_manager));

    let stocks_service = Arc::new(StocksService::new(
        Arc::clone(&storage_service),
        provider_service,
        // Arc::new(agent_service),
        embedding_client.clone(),
    ));

    // agent service
    let agent_service = AgentService::new();
    let tools_service = ToolsService::new(
        stocks_service.clone(),
        embedding_client.clone(),
        Arc::new(agent_service),
        openai_api_key,
        gemini_api_key,
        anthropic_api_key,
    );

    // application state
    let app_state = AppState {
        tools_service: Arc::new(tools_service),
        stocks_service: stocks_service.clone(),
    };

    let admin_routes =
        Router::new().route("/load-tickers", post(handlers::admin::load_tickers_handler));

    let cron_routes = Router::new()
        .route(
            "/update-tickers-eod",
            get(handlers::cron::handle_tickers_eod),
        )
        .route(
            "/update-ticker-embeddings-eod",
            get(handlers::cron::handle_ticker_embeddings_eod),
        )
        // Apply the protection only to these routes
        .layer(axum::middleware::from_fn(middleware::guard_cron_request));

    let origins = [
        "http://localhost:4200".parse::<HeaderValue>().unwrap(),
        "http://localhost:4201".parse::<HeaderValue>().unwrap(),
        "http://localhost:4202".parse::<HeaderValue>().unwrap(),
    ];
    let cors = CorsLayer::new()
        .allow_origin(origins)
        // .allow_origin("http://localhost:4201".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    // Router
    let app = Router::new()
        .nest("/cron", cron_routes)
        .nest("/admin", admin_routes)
        .route("/tickers", get(get_tickers))
        .route("/tickers/{symbol}/history", get(get_ticker_history))
        .route("/tickers/{symbol}/history_latest", get(get_ticker_history_latest))
        .route("/tickers/{symbol}/indicators_latest", get(get_ticker_indicators_latest))
        .route("/tickers/{symbol}/sentiments", get(get_ticker_sentiments))
        .route("/tickers/{symbol}/embeddings", get(get_ticker_embeddings))
        // .route("/tickers/{symbol}/update_embeddings", get(handle_embeddings_eod_update))
        // .route("/tickers/load", get(load_tickers_handler))
        .route("/tickers/screen", post(screen_tickers_handler))
        .route("/tickers/search", post(search_tickers_handler))
        .route("/tickers/analyse", post(analyse_tickers_handler))
        .route("/tickers/update-eod", get(handle_tickers_eod))
        .route("/tickers/update-ticker-embeddings-eod", get(handle_ticker_embeddings_eod))
        .route("/tickers/update-ticker-predictions-eod", get(handle_ticker_predictions_eod))
        .layer(cors)
        .with_state(app_state) // Shared state
        ;

    let listener = TcpListener::bind("127.0.0.1:3002").await.unwrap();
    println!("🚀 Listening on http://127.0.0.1:3002");

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
