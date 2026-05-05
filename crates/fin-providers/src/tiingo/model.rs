use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TiingoTickerPriceData {
    pub ticker: String,
    #[serde(rename = "priceData")]
    pub price_data: Vec<TiingoTickerHistory>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TiingoTickerRealtime {
    pub ticker: String,
    #[serde(rename = "timestamp")]
    pub date: DateTime<Utc>,
    #[serde(rename = "tngoLast")]
    pub tngo_last: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TiingoTickerHistory {
    pub date: DateTime<Utc>,
    pub close: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub open: Decimal,
    pub volume: Decimal,
    #[serde(rename = "adjClose", default)]
    pub adj_close: Decimal,
    #[serde(rename = "adjHigh", default)]
    pub adj_high: Decimal,
    #[serde(rename = "adjLow", default)]
    pub adj_low: Decimal,
    #[serde(rename = "adjOpen", default)]
    pub adj_open: Decimal,
    #[serde(rename = "adjVolume", default)]
    pub adj_volume: Decimal,
    #[serde(rename = "divCash", default)]
    pub div_cash: Decimal,
    #[serde(rename = "splitFactor", default)]
    pub split_factor: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TiingoTickerNews {
    #[serde(rename = "publishedDate", default)]
    pub date: DateTime<Utc>,
    pub url: String,
    pub title: String,
    pub description: Option<String>,
    pub source: String,
}
