use chrono::{DateTime, Utc};
use serde::Serialize;
use storage_core::core::RepoModel;

use crate::ticker::{TICKER_CONTROL_COLLECTION_NAME, TickerSeed};


#[derive(Serialize, serde::Deserialize, Clone, Debug)]
pub struct TickerControl {
    pub id: String,
    pub symbol: String,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_history_sync_at: Option<DateTime<Utc>>,
    pub last_indicator_sync_at: Option<DateTime<Utc>>,
    pub last_sentiment_sync_at: Option<DateTime<Utc>>,    
    pub last_embedding_sync_at: Option<DateTime<Utc>>,
}


impl RepoModel<String> for TickerControl {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn collection(&self) -> &'static str {
        TICKER_CONTROL_COLLECTION_NAME
    }
}

impl TickerControl {
    pub fn new(seed: TickerSeed) -> Self {
        TickerControl{
            id: seed.symbol.clone(),
            symbol: seed.symbol,
            last_history_sync_at: None,
            last_indicator_sync_at: None,
            last_sentiment_sync_at: None,
            last_sync_at: None,
            last_embedding_sync_at: None
        }
    }
}