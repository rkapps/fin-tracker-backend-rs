use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TickerParam {
    pub symbol: String,
}
