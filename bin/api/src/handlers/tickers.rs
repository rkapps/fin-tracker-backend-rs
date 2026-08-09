use anyhow::Result;
use axum::{
    Json,
    extract::{Path, State},
};
use reqwest::StatusCode;
use rustic_finance::{
    domain::{
        TickerEntity, TickerGroup, TickerNewsEntity,
        dto::{ticker_chart_entity::TickerChartEntity, ticker_search_param::TickerSearchParam},
    },
    service::FinanceService,
};
use serde::Deserialize;
use serde_with::{StringWithSeparator, formats::CommaSeparator, serde_as};
use std::sync::Arc;

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
    State(ticker_service): State<Arc<FinanceService>>,
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

pub async fn get_ticker_groups_handler(
    State(ticker_service): State<Arc<FinanceService>>,
) -> Result<Json<Vec<TickerGroup>>, (StatusCode, String)> {
    let groups = ticker_service
        .get_ticker_groups()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("{}", e)))?;

    Ok(Json(groups))
}

pub async fn get_ticker_news_handler(
    State(ticker_service): State<Arc<FinanceService>>,
    Path(symbol): Path<String>,
) -> Result<Json<Vec<TickerNewsEntity>>, (StatusCode, String)> {
    let news = ticker_service
        .get_ticker_news(&symbol.to_uppercase())
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("{}", e)))?;
    Ok(Json(news))
}

pub async fn search_tickers_handler(
    State(ticker_service): State<Arc<FinanceService>>,
    Json(param): Json<TickerSearchParam>,
) -> Result<Json<Vec<TickerEntity>>, (StatusCode, String)> {
    let tickers = ticker_service
        .search_tickers(param)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("{}", e)))?;

    Ok(Json(tickers))
}

