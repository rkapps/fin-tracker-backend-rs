use std::{collections::HashMap, string};

use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CmcCryptoData {
    pub data: HashMap<String, Vec<CmcCryptoSymbol>>
}

#[derive(Debug, Deserialize)]
pub struct CmcCryptoSymbol {
    pub id: i32,
    pub name: String,
    pub symbol: String,
    pub slug: String,
    pub total_supply: Option<f64>,
    pub is_active: i8,
    pub last_updated: String,
    pub quote: HashMap<String, CmcCryptoQuote>

}


#[derive(Debug, Deserialize)]
pub struct CmcCryptoQuote {
    pub price: Option<f64>,
    pub market_cap: Option<f64>
}

