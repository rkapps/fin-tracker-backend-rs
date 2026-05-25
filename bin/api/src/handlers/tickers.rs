use anyhow::Result;
use axum::{
    Json,
    extract::{Path, Query, State},
};
use fin_domain::{
    dto::{
        ticker_chart_entity::TickerChartEntity, ticker_entity::TickerEntity,
        ticker_group::TickerGroup, ticker_indicator_entity::TickerIndicatorEntity,
        ticker_news_entity::TickerNewsEntity, ticker_peer::TickerPeer,
        ticker_search_param::TickerSearchParam, ticker_sentiment_entity::TickerSentimentEntity,
        ticker_snapshot::TickerSnapshot,
    },
    tickers::{TickerEmbedding, TickerSentiment},
    utils::dec_utils::decimal_to_float,
};
use fin_services::ticker::TickersService;
use reqwest::StatusCode;
use rust_decimal::{Decimal, prelude::FromPrimitive};
use serde::Deserialize;
use serde_with::{StringWithSeparator, formats::CommaSeparator, serde_as};
use std::sync::Arc;
use tracing::debug;

// #[serde_as]
// #[derive(Deserialize, Debug)]
// pub struct TickerSearchQueryParam {
//     #[serde_as(as = "Option<StringWithSeparator<CommaSeparator, String>>")]
//     pub symbols: Option<Vec<String>>,
//     pub function: Option<String>,
// }

#[serde_as]
#[derive(Deserialize)]
pub struct TickerPeersQueryParam {
    #[serde_as(as = "Option<StringWithSeparator<CommaSeparator, String>>")]
    pub symbols: Option<Vec<String>>,
    pub limit: Option<usize>,
}

#[serde_as]
#[derive(Deserialize)]
pub struct TickerSymbolsParam {
    #[serde_as(as = "Option<StringWithSeparator<CommaSeparator, String>>")]
    pub symbols: Option<Vec<String>>,
}

#[serde_as]
#[derive(Deserialize)]
pub struct TickerSentimentsParam {
    #[serde_as(as = "Option<StringWithSeparator<CommaSeparator, String>>")]
    pub symbols: Option<Vec<String>>,
    pub relevance_score: Option<f64>,
}

#[serde_as]
#[derive(Deserialize)]
pub struct TickerIndicatorsQueryParam {
    #[serde_as(as = "Option<StringWithSeparator<CommaSeparator, String>>")]
    pub symbols: Option<Vec<String>>,
    pub limit: Option<usize>,
}

pub async fn get_ticker_charts_handler(
    State(ticker_service): State<Arc<TickersService>>,
    Path(symbol): Path<String>,
) -> Result<Json<Vec<TickerChartEntity>>, (StatusCode, String)> {
    let charts = ticker_service
        .get_ticker_charts(&symbol.to_uppercase())
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Get Ticker History error: {}", e),
            )
        })?;
    Ok(Json(charts))
}

pub async fn get_ticker_peers_by_symbols_handler(
    State(ticker_service): State<Arc<TickersService>>,
    Query(param): Query<TickerPeersQueryParam>,
) -> Result<Json<Vec<TickerPeer>>, (StatusCode, String)> {
    let peers = ticker_service
        .get_ticker_peers_for_symbols(
            param.symbols.unwrap_or_default(),
            param.limit.unwrap_or(100),
        )
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("{}", e)))?;

    Ok(Json(peers))
}

pub async fn get_ticker_snapshots_by_symbols_handler(
    State(ticker_service): State<Arc<TickersService>>,
    Query(param): Query<TickerSymbolsParam>,
) -> Result<Json<Vec<TickerSnapshot>>, (StatusCode, String)> {
    let tickers = ticker_service
        .storage_service
        .get_tickers_by_symbols(param.symbols.unwrap_or_default())
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("{}", e)))?;

    let snapshots: Vec<TickerSnapshot> = tickers.into_iter().map(TickerSnapshot::from).collect();
    Ok(Json(snapshots))
}

pub async fn get_ticker_sentiments_by_symbols_handler(
    State(ticker_service): State<Arc<TickersService>>,
    Query(param): Query<TickerSentimentsParam>,
) -> Result<Json<Vec<TickerSentiment>>, (StatusCode, String)> {
    let relevance_score = param.relevance_score.unwrap_or_default();
    let relevance_score = Decimal::from_f64(relevance_score).unwrap_or_default();
    let sentiments = ticker_service
        .storage_service
        .get_ticker_sentiments_with_score(param.symbols.unwrap_or_default(), &relevance_score)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("{}", e)))?;

    Ok(Json(sentiments))
}

pub async fn get_ticker_embeddings_by_symbols_handler(
    State(ticker_service): State<Arc<TickersService>>,
    Query(param): Query<TickerSymbolsParam>,
) -> Result<Json<Vec<TickerEmbedding>>, (StatusCode, String)> {
    let embeddings = ticker_service
        .storage_service
        .get_ticker_embeddings(param.symbols.unwrap_or_default())
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("{}", e)))?;
    debug!("embeddings: {}", embeddings.len());
    Ok(Json(embeddings))
}

pub async fn get_ticker_indicators_by_symbols_handler(
    State(ticker_service): State<Arc<TickersService>>,
    Query(param): Query<TickerIndicatorsQueryParam>,
) -> Result<Json<Vec<TickerIndicatorEntity>>, (StatusCode, String)> {
    let indicators = ticker_service
        .storage_service
        .get_ticker_indicators_by_symbols(param.symbols.unwrap_or_default(), param.limit)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("{}", e)))?;

    Ok(Json(indicators))
}

pub async fn get_ticker_groups_handler(
    State(ticker_service): State<Arc<TickersService>>,
) -> Result<Json<Vec<TickerGroup>>, (StatusCode, String)> {
    let groups = ticker_service
        .get_ticker_groups()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("{}", e)))?;

    Ok(Json(groups))
}

pub async fn get_ticker_news_handler(
    State(ticker_service): State<Arc<TickersService>>,
    Path(symbol): Path<String>,
) -> Result<Json<Vec<TickerNewsEntity>>, (StatusCode, String)> {
    let news = ticker_service
        .get_ticker_news(&symbol.to_uppercase())
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("{}", e)))?;
    Ok(Json(news))
}

pub async fn search_tickers_handler(
    State(ticker_service): State<Arc<TickersService>>,
    Json(param): Json<TickerSearchParam>,
) -> Result<Json<Vec<TickerEntity>>, (StatusCode, String)> {
    let tickers = ticker_service
        .search_tickers(param)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("{}", e)))?;

    Ok(Json(tickers))
}

pub async fn search_tickers_sentiments_handler(
    State(ticker_service): State<Arc<TickersService>>,
    Json(param): Json<TickerSearchParam>,
) -> Result<Json<Vec<TickerSentimentEntity>>, (StatusCode, String)> {
    let sentiments = ticker_service
        .search_ticker_sentiments(param)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("{}", e)))?;

    Ok(Json(sentiments))
}
