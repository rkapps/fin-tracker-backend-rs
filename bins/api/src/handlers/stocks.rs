use std::sync::Arc;

use agentic_core::{
    agent::service::AgentService, capabilities::completion::response::CompletionResponseContent,
};
use axum::{
    Json,
    extract::{Path, State},
};
use fin_domain::ticker::{
    Ticker, TickerEmbedding, TickerHistory, TickerIndicator, TickerSentiment,
};
use fin_services::stocks::StocksService;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::state::{AnthropicApiKey, GeminiApiKey, OpenAIApiKey};

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

pub async fn get_tickers(
    State(stocks_service): State<Arc<StocksService>>,
) -> Result<Json<Vec<Ticker>>, (StatusCode, String)> {
    let tickers = stocks_service
        .storage_service
        .get_tickers()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Get Ticker error: {}", e)))?;

    Ok(Json(tickers))
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
) -> Result<Json<Vec<TickerIndicator>>, (StatusCode, String)> {
    let indicators = stocks_service
        .storage_service
        .get_ticker_indicators_latest(&symbol)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Get Ticker History error: {}", e),
            )
        })?;

    debug!("Indicators: {:#?}", indicators.len());
    Ok(Json(indicators))
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

pub async fn analyse_tickers_handler(
    State(stocks_service): State<Arc<StocksService>>,
    State(agent_service): State<Arc<AgentService>>,
    State(OpenAIApiKey(openai_api_key)): State<OpenAIApiKey>,
    State(GeminiApiKey(gemini_api_key)): State<GeminiApiKey>,
    State(AnthropicApiKey(anthropic_api_key)): State<AnthropicApiKey>,
    Json(param): Json<TickerAnalyseParam>,
) -> Result<Json<TickerAnalyseResponse>, (StatusCode, String)> {
    debug!("analyse params: {:?}", param);
    let response_id = param.prev_response_id;

    let response = stocks_service
        .analyse_tickers(
            agent_service,
            &openai_api_key,
            &gemini_api_key,
            &anthropic_api_key,
            &param.prompt,
            response_id,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Analyse ticker error: {}", e),
            )
        })?;

    let aresponse = response
        .contents
        .iter()
        .find_map(|c| {
            if let CompletionResponseContent::Text(val) = c {
                Some(val.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();

    let aresponse = TickerAnalyseResponse {
        content: aresponse,
        response_id: response.response_id,
    };
    Ok(Json(aresponse))
    // Ok(Json(response))
}
