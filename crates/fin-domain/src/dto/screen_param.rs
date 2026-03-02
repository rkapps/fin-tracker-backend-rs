use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TickerScreenParam {
    pub query: Option<String>, // semantic: "cloud security", "payments infrastructure"
    pub signals: Option<Vec<String>>, // ["RSI Oversold", "MACD Bullish Crossover"]
    pub industry: Option<String>, // regex match
    pub market_cap_range: Option<String>, // "mega", "large", "mid", "small"
    pub asset_type: Option<String>, // "stock", "etf"
    pub limit: Option<usize>,
}
