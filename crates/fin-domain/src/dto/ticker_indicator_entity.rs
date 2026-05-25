use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TickerIndicatorEntity {
    pub id: String,
    pub symbol: String,
    pub rsi_14: f64,
    pub sma_50: f64,
    pub sma_200: f64,
    pub macd: f64,
    pub macd_signal: f64,
    pub bb_upper: f64,
    pub bb_lower: f64,
}
