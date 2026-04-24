use anyhow::Result;
use axum::{
    Json,
    extract::{Path, State},
};
use fin_domain::dto::{
    ticker_chart_entity::TickerChartEntity, ticker_entity::TickerEntity,
    ticker_news_entity::TickerNewsEntity, ticker_search_param::TickerSearchParam,
};
use fin_services::ticker::TickersService;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_with::{StringWithSeparator, formats::CommaSeparator, serde_as};
use std::{collections::HashMap, sync::Arc};

#[serde_as]
#[derive(Deserialize, Debug)]
pub struct TickerQueryParam {
    #[serde_as(as = "Option<StringWithSeparator<CommaSeparator, String>>")]
    pub symbols: Option<Vec<String>>,
    pub function: Option<String>,
}

pub async fn get_ticker_charts_handler(
    State(ticker_service): State<Arc<TickersService>>,
    Path(symbol): Path<String>,
) -> Result<Json<Vec<TickerChartEntity>>, (StatusCode, String)> {
    let charts = ticker_service
        .get_ticker_charts(&symbol)
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
    State(ticker_service): State<Arc<TickersService>>,
) -> Result<Json<HashMap<String, Vec<String>>>, (StatusCode, String)> {
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
        .get_ticker_news(&symbol)
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
