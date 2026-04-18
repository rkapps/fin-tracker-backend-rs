use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TickerSearchParam {
    // selection
    pub symbols: Option<String>,
    pub function: Option<String>,

    // search
    pub asset_type: Option<String>, // "stock", "etf"
    pub query: Option<String>, // semantic: "cloud security", "payments infrastructure"
    pub signals: Option<Vec<String>>, // ["RSI Oversold", "MACD Bullish Crossover"]
    pub industry: Option<String>, // regex match
    pub market_cap_range: Option<String>, // "mega", "large", "mid", "small"
    pub r#yield: Option<f32>,

    // sorting
    pub sort_by: Option<String>,        // "price", "change_pct", "volume", "market_cap"
    pub sort_dir: Option<String>,       // "asc", "desc"

    // pagination
    pub limit: Option<usize>,

}

// impl TickerSearchParam {
//     pub fn new_for_asset_type(asset_type: &str) -> TickerSearchParam {
//         TickerSearchParam {
//             asset_type: Some(asset_type.to_string()),
//             industry: None,
//             limit: None,
//             market_cap_range: None,
//             query: None,
//             signals: None,
//             r#yield: None,
//         }
//     }
// }