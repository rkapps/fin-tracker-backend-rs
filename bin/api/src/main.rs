use std::sync::Arc;

use anyhow::Result;
use axum::{
    Router,
    http::HeaderValue,
    routing::{get, post},
};

use bin_shared::{
    logger::set_logger,
    services::{get_analyse_service, get_ticker_service},
};
use fin_tracker_api::{
    handlers::{
        analyse::{analyse_tickers_handler, analyse_tickers_streaming_handler},
        tickers::{
            get_ticker_charts_handler, get_ticker_groups_handler, get_tickers_handler,
            search_tickers_handler,
        },
    },
    state::AppState,
};
use reqwest::Method;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

#[tokio::main]

async fn main() -> Result<()> {
    set_logger();

    // let filter = std::env::var("RUST_LOG")
    //     .unwrap_or_else(|_| "fin_services=trace,fin_core=debug,agentic_core=info".to_string());

    // println!("{}", filter);
    // let subscriber = FmtSubscriber::builder()
    // .with_max_level(Level::TRACE)
    // .with_target(true)
    // .with_line_number(true)
    // .with_env_filter(filter)
    // .compact()
    // .json()
    // .finish();

    // tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    // let finance_db_path =
    //     env::var("FINANCE_DB_PATH").expect("FINANCE_DB_PATH not found in environment variables.");
    // let finance_db_name =
    //     env::var("FINANCE_DB_NAME").expect("FINANCE_DB_NAME not found in environment variables.");
    // let alpha_key =
    //     env::var("ALPHA_API_KEY").expect("ALPHA_API_KEY not found in environment variables.");
    // let tiingo_token =
    //     env::var("TIINGO_API_TOKEN").expect("TIINGO_API_TOKEN not found in environment variables.");
    // let openai_api_key: String =
    //     env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");
    // let gemini_api_key: String =
    //     env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY environment variable not set");
    // let anthropic_api_key: String =
    //     env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY environment variable not set");

    // stocks
    // let embedding_client = Arc::new(OpenAIEmbeddingClient::new(&openai_api_key)?);
    // let provider_service = fin_providers::ProviderService::new(&alpha_key, &tiingo_token)?;
    // let stocks_storage = FileStorageManager::new(finance_db_name, finance_db_path).await?;

    // let mongo_uri = env::var("MONGO_ATLAS_CONN_STR")
    //     .expect("MONGO_ATLAS_CONN_STR envrionment variable not set");
    // println!("MongoAtlas Uri: {}", mongo_uri);
    // let storage_manager = MongoStorageManager::new(&mongo_uri, "test").await?;

    // // 2. Wrap in storage service (business logic layer)
    // let storage_service: Arc<dyn StorageService> =
    //     Arc::new(MongoStorageService::new(storage_manager));

    // let ml_service = MlService::new(Arc::clone(&storage_service));
    // let ticker_service = TickerService::new(Arc::clone(&storage_service));

    // let stocks_service = Arc::new(StocksService::new(
    //     Arc::clone(&storage_service),
    //     provider_service,
    //     // Arc::new(agent_service),
    //     embedding_client.clone(),
    //     ml_service.clone(),
    // ));

    // // agent service
    // let agent_service = AgentService::new();
    // let tools_service = ToolsService::new(
    //     stocks_service.clone(),
    //     embedding_client.clone(),
    //     Arc::new(agent_service),
    //     openai_api_key,
    //     gemini_api_key,
    //     anthropic_api_key,
    // );

    let ticker_service = get_ticker_service().await?;
    let analyse_service = get_analyse_service().await?;
    // application state
    let app_state = AppState {
        ticker_service: Arc::new(ticker_service),
        analyse_service: Arc::new(analyse_service),
        // tools_service: Arc::new(tools_service),
        // stocks_service: stocks_service.clone(),
        // ml_service: Arc::new(ml_service),
    };

    // let admin_routes =
    //     Router::new().route("/load-tickers", post(handlers::admin::load_tickers_handler));

    // let cron_routes = Router::new()
    //     .route("/update-tickers-eod", post(handle_tickers_eod))
    //     .route(
    //         "/update-ticker-embeddings-eod",
    //         post(handle_ticker_embeddings_eod),
    //     )
    //     .route(
    //         "/build_training_model",
    //         post(handle_build_tickers_training_model),
    //     )
    //     .route(
    //         "/update-ticker-predictions-eod",
    //         post(handle_ticker_prediction_signals_eod),
    //     )
    //     // Apply the protection only to these routes
    //     .layer(axum::middleware::from_fn(middleware::guard_cron_request));

    let origins = [
        "http://localhost:4200".parse::<HeaderValue>().unwrap(),
        "http://localhost:4201".parse::<HeaderValue>().unwrap(),
        "http://localhost:4202".parse::<HeaderValue>().unwrap(),
    ];
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    // Router
    let app = Router::new()
        // .nest("/cron", cron_routes)
        // .nest("/admin", admin_routes)
        .route("/tickers", get(get_tickers_handler))
        .route("/tickers/groups", get(get_ticker_groups_handler))
        // .route("/tickers/{symbol}/history", get(get_ticker_history))
        .route("/tickers/{symbol}/charts", get(get_ticker_charts_handler))
        // .route("/tickers/{symbol}/history_latest", get(get_ticker_history_latest))
        // .route("/tickers/{symbol}/indicators_latest", get(get_ticker_indicators_latest))
        // .route("/tickers/{symbol}/sentiments", get(get_ticker_sentiments))
        // .route("/tickers/{symbol}/embeddings", get(get_ticker_embeddings))
        // .route("/tickers/{symbol}/update_embeddings", get(handle_embeddings_eod_update))
        // .route("/tickers/load", get(load_tickers_handler))
        // .route("/tickers/screen", post(screen_tickers_handler))
        .route("/tickers/search", post(search_tickers_handler))
        .route("/tickers/analyse", post(analyse_tickers_handler))
        .route("/tickers/analyse_streaming", post(analyse_tickers_streaming_handler))

        .layer(cors)
        .with_state(app_state) // Shared state
        ;

    // let listener = TcpListener::bind("127.0.0.1:3002").await.unwrap();
    // println!("🚀 Listening on http://127.0.0.1:3002");
    let port = std::env::var("PORT").unwrap_or_else(|_| "3002".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("🚀 Listening on {:?}", listener);

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
