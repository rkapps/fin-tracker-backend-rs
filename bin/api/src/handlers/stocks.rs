use axum::{
    Json,
    extract::{Path, State},
};
use fin_domain::{
    dto::screen_param::TickerScreenParam,
    ticker::{Ticker, TickerEmbedding, TickerHistory, TickerIndicator, TickerSentiment},
};
use fin_services::stocks::StocksService;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;


#[derive(Deserialize, Debug)]
pub struct TickerAnalyseParam {
    pub prompt: String,
    pub prev_response_id: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct TickerAnalyseResponse {
    pub content: String,
    pub response_id: String,
}


pub async fn get_ticker_history(
    State(stocks_service): State<Arc<StocksService>>,
    Path(symbol): Path<String>,
) -> Result<Json<Vec<TickerHistory>>, (StatusCode, String)> {
    let histories = stocks_service
        .storage_service
        .get_ticker_history(&symbol)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Get Ticker History error: {}", e),
            )
        })?;
    Ok(Json(histories))
}

pub async fn get_ticker_history_latest(
    State(stocks_service): State<Arc<StocksService>>,
    Path(symbol): Path<String>,
) -> Result<Json<Vec<TickerHistory>>, (StatusCode, String)> {
    let histories = stocks_service
        .storage_service
        .get_ticker_history_latest(&symbol)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Get Ticker History error: {}", e),
            )
        })?;
    Ok(Json(histories))
}

pub async fn get_ticker_indicators_latest(
    State(stocks_service): State<Arc<StocksService>>,
    Path(symbol): Path<String>,
) -> Result<Json<TickerIndicator>, (StatusCode, String)> {
    let indicator = stocks_service
        .storage_service
        .get_ticker_indicators_latest(&symbol)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Get Ticker History error: {}", e),
            )
        })?;

    Ok(Json(indicator))
}

pub async fn get_ticker_sentiments(
    State(stocks_service): State<Arc<StocksService>>,
    Path(symbol): Path<String>,
) -> Result<Json<Vec<TickerSentiment>>, (StatusCode, String)> {
    let sentiments = stocks_service
        .storage_service
        .get_ticker_sentiments(&symbol)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Get Ticker Sentiment error: {}", e),
            )
        })?;
    Ok(Json(sentiments))
}

pub async fn get_ticker_embeddings(
    State(stocks_service): State<Arc<StocksService>>,
    Path(symbol): Path<String>,
) -> Result<Json<Vec<TickerEmbedding>>, (StatusCode, String)> {
    let embeddings = stocks_service
        .storage_service
        .get_ticker_embeddings(&symbol)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Get Ticker Embedding error: {}", e),
            )
        })?;
    Ok(Json(embeddings))
}

pub async fn screen_tickers_handler(
    State(stocks_service): State<Arc<StocksService>>,
    Json(param): Json<TickerScreenParam>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    debug!("screen params: {:?}", param);

    let tickers = stocks_service.screen_tickers(param).await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Search ticker error: {}", e),
        )
    })?;
    Ok(Json(tickers))
    // Ok(Json(response))
}
pub async fn search_tickers_handler(
    State(stocks_service): State<Arc<StocksService>>,
    Json(param): Json<TickerScreenParam>,
) -> Result<Json<Vec<Ticker>>, (StatusCode, String)> {
    debug!("search params: {:?}", param);

    let tickers = stocks_service
        .storage_service
        .search_tickers(param)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Search ticker error: {}", e),
            )
        })?;
    Ok(Json(tickers))
    // Ok(Json(response))
}
