use std::sync::Arc;

use anyhow::Result;
use axum::{
    Router,
    http::HeaderValue,
    routing::{get, post},
};

use bin_shared::{
    config::loader::load_app_config, logger::set_logger, services::{get_agent_service, get_analyse_service, get_tickers_service, load_agent_config}
};
use fin_tracker_api::{
    handlers::{
        analyse::{analyse_tickers_handler, analyse_tickers_streaming_handler, get_llm_providers_handler},
        tickers::{get_ticker_charts_handler, get_ticker_groups_handler, get_ticker_news_handler, search_tickers_handler},
    },
    state::AppState,
};
use reqwest::Method;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

#[tokio::main]

async fn main() -> Result<()> {
    set_logger();

    let app_config = load_app_config().await?;
    let agent_config = load_agent_config(&app_config).await;
    let agent_service = get_agent_service(agent_config)?;
    let analyse_service = get_analyse_service(agent_service).await?;
    let ticker_service = get_tickers_service().await?;
    
    // application state
    let app_state = AppState {
        ticker_service: Arc::new(ticker_service),
        analyse_service: Arc::new(analyse_service),
    };

    let origins = [
        "http://localhost:4200".parse::<HeaderValue>().unwrap(),
        "http://localhost:4201".parse::<HeaderValue>().unwrap(),
        "http://localhost:4202".parse::<HeaderValue>().unwrap(),
        "https://fin-tracker-rkapps.web.app".parse::<HeaderValue>().unwrap(),
    ];
    
    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
        ]);

    // Router
    let app = Router::new()
        // .route("/tickers", get(get_tickers_handler))
        .route("/tickers/groups", get(get_ticker_groups_handler))
        // .route("/tickers/{symbol}/history", get(get_ticker_history))
        .route("/tickers/{symbol}/charts", get(get_ticker_charts_handler))
        .route("/tickers/{symbol}/news", get(get_ticker_news_handler))
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
        .route("/llm_providers", get(get_llm_providers_handler))

        .layer(cors)
        .with_state(app_state) // Shared state
        ;

    let port = std::env::var("PORT").unwrap_or_else(|_| "3002".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await.unwrap();
    println!("🚀 Listening on {:?}", listener);

    axum::serve(listener, app).await.unwrap();

    Ok(())
}
