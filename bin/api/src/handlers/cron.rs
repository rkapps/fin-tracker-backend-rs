use std::sync::Arc;

use axum::extract::State;
use fin_services::{ml::service::MlService, stocks::StocksService};
use reqwest::StatusCode;
use tracing::{error, info};

pub async fn handle_tickers_eod(State(stocks_service): State<Arc<StocksService>>) -> StatusCode {
    // 1. Clone the Arc so the background task owns a handle to the service
    let service_clone = stocks_service.clone();

    tokio::spawn(async move {
        match service_clone.handle_tickers_eod().await {
            Ok(_) => info!("Background Tickers EOD Update completed successfully."),
            Err(e) => error!("Background Tickers EOD Update failed: {:?}", e),
        }
    });

    StatusCode::ACCEPTED
}

pub async fn handle_ticker_embeddings_eod(
    State(stocks_service): State<Arc<StocksService>>,
) -> StatusCode {
    // 1. Clone the Arc so the background task owns a handle to the service
    let service_clone = stocks_service.clone();

    // 2. Spawn the task onto the Tokio runtime
    tokio::spawn(async move {
        match service_clone.handle_ticker_embeddings_eod().await {
            Ok(_) => info!("Background Ticker Embeddings EOD Update completed successfully."),
            Err(e) => error!("Background Ticker Embeddings EOD Update failed: {:?}", e),
        }
    });

    StatusCode::ACCEPTED
}

pub async fn handle_ticker_prediction_signals_eod(
    State(stocks_service): State<Arc<StocksService>>,
) -> StatusCode {
    // 1. Clone the Arc so the background task owns a handle to the service
    let service_clone = stocks_service.clone();

    // 2. Spawn the task onto the Tokio runtime
    tokio::spawn(async move {
        match service_clone.handle_ticker_prediction_signals_eod().await {
            Ok(_) => info!("Background Ticker Predictions EOD Update completed successfully."),
            Err(e) => error!("Background Ticker Predictions EOD Update failed: {:?}", e),
        }
    });

    StatusCode::ACCEPTED
}

pub async fn handle_build_tickers_training_model(
    State(ml_service): State<Arc<MlService>>,
) -> StatusCode {
    // 1. Clone the Arc so the background task owns a handle to the service
    let ml_service_clone = ml_service.clone();

    // 2. Spawn the task onto the Tokio runtime
    tokio::spawn(async move {
        match ml_service_clone.build_and_train_all().await {
            Ok(_) => info!("Background Ticker Training Model EOD Update completed successfully."),
            Err(e) => error!(
                "Background Ticker Training Model EOD Update failed: {:?}",
                e
            ),
        }
    });

    StatusCode::ACCEPTED
}
