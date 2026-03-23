use std::sync::Arc;

use axum::{Json, extract::State};
use fin_domain::ticker::TickerSeed;
use fin_services::stocks::StocksService;
use reqwest::StatusCode;
use tracing::{error, info};



pub async fn load_tickers_handler(
    State(stocks_service): State<Arc<StocksService>>,
    Json(ticker_seeds): Json<Vec<TickerSeed>>,
) -> StatusCode {

    // 1. Clone the Arc so the background task owns a handle to the service
    let service_clone = stocks_service.clone();

    // 2. Spawn the task onto the Tokio runtime
    tokio::spawn(async move {
        match service_clone.load_tickers(ticker_seeds).await {
            Ok(_) => info!("Background Ticker Load completed successfully."),
            Err(e) => error!("Background Ticker Load failed: {:?}", e),
        }
    });

    StatusCode::ACCEPTED
}

