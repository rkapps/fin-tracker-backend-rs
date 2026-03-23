use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use fin_domain::dto::{ticker_chart_entity::TickerChartEntity, ticker_entity::TickerEntity};
use fin_services::ticker_service::TickerService;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_with::{StringWithSeparator, formats::CommaSeparator, serde_as};

#[serde_as]
#[derive(Deserialize, Debug)]
pub struct TickerQueryParam {
    #[serde_as(as = "Option<StringWithSeparator<CommaSeparator, String>>")]
    pub symbols: Option<Vec<String>>,
    pub function: Option<String>,
}

pub async fn get_tickers(
    State(ticker_service): State<Arc<TickerService>>,
    Query(param): Query<TickerQueryParam>,
) -> Result<Json<Vec<TickerEntity>>, (StatusCode, String)> {
    let tickers: Vec<TickerEntity> = match (param.symbols, param.function) {
        (Some(s), None) if !s.is_empty() => ticker_service
            .get_tickers_by_symbols(s)
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Get Ticker error: {}", e)))?,
        (_, Some(f)) => ticker_service
            .get_tickers_by_function(&f)
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Get Ticker error: {}", e)))?,
        (_, None) => {
            // Return error if both are present
            return Err((
                StatusCode::BAD_REQUEST,
                "Cannot use 'symbols' and 'function' together".to_string(),
            ));
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "Must provide 'symbols' or 'function'".to_string(),
            ));
        }
    };

    Ok(Json(tickers))
}

pub async fn get_ticker_charts(
    State(ticker_service): State<Arc<TickerService>>,
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


// pub async fn get_tickers_by_symbols(
//     State(ticker_service): State<Arc<TickerService>>,
//     Query(param): Query<TickerQueryParam>,
// ) -> Result<Json<Vec<TickerEntity>>, (StatusCode, String)> {
//     let tickers = ticker_service
//         .get_tickers_by_symbols(param.symbols)
//         .await
//         .map_err(|e| (StatusCode::BAD_REQUEST, format!("Get Ticker error: {}", e)))?;

//     Ok(Json(tickers))
// }

// pub async fn get_tickers_by_function(
//     State(ticker_service): State<Arc<TickerService>>,
//     Query(param): Query<TickerQueryParam>,
// ) -> Result<Json<Vec<TickerEntity>>, (StatusCode, String)> {
//     let tickers = ticker_service
//         .get_tickers_by_symbols(param.symbols)
//         .await
//         .map_err(|e| (StatusCode::BAD_REQUEST, format!("Get Ticker error: {}", e)))?;

//     Ok(Json(tickers))
// }
