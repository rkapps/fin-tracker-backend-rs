use crate::tickers::AssetType;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TickerSeed {
    pub asset_type: AssetType,
    pub exchange: String,
    pub symbol: String,
    pub name: String,
    pub sector: String,
    pub industry: String,
    pub overview: String,
}
