use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TickerPeer {
    pub symbol: String,
    pub peers: Vec<PeerScore>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PeerScore {
    pub symbol: String,
    pub score: i32,
}
