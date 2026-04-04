use serde::{Deserialize, Serialize};

use crate::ticker::Ticker;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TickerScreenParam {
    pub query: Option<String>, // semantic: "cloud security", "payments infrastructure"
    pub signals: Option<Vec<String>>, // ["RSI Oversold", "MACD Bullish Crossover"]
    pub industry: Option<String>, // regex match
    pub market_cap_range: Option<String>, // "mega", "large", "mid", "small"
    pub asset_type: Option<String>, // "stock", "etf"
    pub limit: Option<usize>,
    pub r#yield: Option<f32>,
}

impl TickerScreenParam {

    pub fn new_for_asset_type(asset_type: &str) -> TickerScreenParam {
        TickerScreenParam{
            asset_type: Some(asset_type.to_string()),
            industry: None,
            limit: None,
            market_cap_range: None,
            query: None,
            signals: None,
            r#yield: None
        }

    }
}
