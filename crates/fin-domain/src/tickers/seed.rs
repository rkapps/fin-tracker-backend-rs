use serde::{Deserialize, Serialize};
use crate::tickers::AssetType;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TickerSeed {
    pub asset_type: AssetType,
    // pub active: bool,
    pub exchange: String,
    pub symbol: String,
    pub name: String,
    pub sector: String,
    pub industry: String,
    pub overview: String,
    // pub country: String,
    // pub currency: String,
}