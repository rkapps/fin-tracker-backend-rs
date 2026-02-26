use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TiingoTickerHistory {
    pub date: DateTime<Utc>,
    pub close: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub open: Decimal,
    pub volume: Decimal,
    #[serde(rename = "adjClose")]
    pub adj_close: Decimal,
    #[serde(rename = "adjHigh")]
    pub adj_high: Decimal,
    #[serde(rename = "adjLow")]
    pub adj_low: Decimal,
    #[serde(rename = "adjOpen")]
    pub adj_open: Decimal,
    #[serde(rename = "adjVolume")]
    pub adj_volume: Decimal,
    #[serde(rename = "divCash")]
    pub div_cash: Decimal,
    #[serde(rename = "splitFactor")]
    pub split_factor: Decimal,
}
