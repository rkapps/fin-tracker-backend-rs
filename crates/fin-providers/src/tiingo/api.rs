use crate::tiingo::model::TiingoTickerHistory;
use anyhow::Result;
use chrono::{DateTime, Utc};
use fin_http::HttpClient;
use tracing::debug;

const TIINGO_EOD_URL: &str = "https://api.tiingo.com/tiingo/daily/";
const TIINGO_CRYPTO_URL: &str = "https://api.tiingo.com/tiingo/crypto/prices";


pub async fn get_stock_history(
    http_client: &HttpClient,
    symbol: &str,
    api_token: &str,
    start_date: &DateTime<Utc>,
) -> Result<Vec<TiingoTickerHistory>> {
    let url = format!(
        "{}{}/prices?token={}&startDate={}",
        TIINGO_EOD_URL, symbol, api_token, convert_datetime_utc_to_ymd(start_date)
    );
    debug!("Tiingo daily url: {}", url);
    let headers = reqwest::header::HeaderMap::new();
    let hist = http_client
        .get_request::<Vec<TiingoTickerHistory>>(url, Some(headers))
        .await?;

    Ok(hist)
}

pub async fn get_crypto_history(
    http_client: &HttpClient,
    symbol: &str,
    frequency: &str,
    api_token: &str,
) -> Result<Vec<TiingoTickerHistory>> {
    let url = format!(
        "{}?tickers={}&frequency={}&token={}", TIINGO_CRYPTO_URL, symbol, frequency, api_token);
    debug!("Tiingo crypto url: {}", url);
    let headers = reqwest::header::HeaderMap::new();
    let hist = http_client
        .get_request::<Vec<TiingoTickerHistory>>(url, Some(headers))
        .await?;

    Ok(hist)
}



fn convert_datetime_utc_to_ymd(now: &DateTime<Utc>) -> String {
    let today_utc = now.naive_utc().date();
    today_utc.format("%Y-%m-%d").to_string()
}