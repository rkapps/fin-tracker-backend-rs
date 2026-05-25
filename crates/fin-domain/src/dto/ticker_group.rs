use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TickerGroup {
    pub sector: String,
    pub industry: String,
}
