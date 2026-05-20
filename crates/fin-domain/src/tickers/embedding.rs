use crate::tickers::deserialize_flexible_datetime;
use crate::tickers::serialize_as_bson_datetime;
use chrono::{DateTime, Utc};
use rustic_storage::core::repository::RepoModel;
use rustic_storage::core::repository::VectorEmbedding;
use serde::{Deserialize, Serialize};

use crate::tickers::TICKER_EMBEDDING_COLLECTION_NAME;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TickerEmbedding {
    pub id: String,
    pub symbol: String,

    #[serde(
        deserialize_with = "deserialize_flexible_datetime",
        serialize_with = "serialize_as_bson_datetime"
    )]
    pub date: DateTime<Utc>,
    pub sentiment_id: String,
    pub embedding_text: String,
    pub vector: Vec<f32>,
}

impl RepoModel<String> for TickerEmbedding {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn collection(&self) -> &'static str {
        TICKER_EMBEDDING_COLLECTION_NAME
    }
}

impl TickerEmbedding {
    pub fn new(
        symbol: &str,
        date: DateTime<Utc>,
        sentiment_id: &str,
        embedding_text: &str,
        vector: Vec<f32>,
    ) -> TickerEmbedding {
        TickerEmbedding {
            date,
            embedding_text: embedding_text.to_string(),
            id: TickerEmbedding::embedding_id(symbol, sentiment_id),
            sentiment_id: sentiment_id.to_string(),
            symbol: symbol.to_string(),
            vector,
        }
    }

    fn embedding_id(symbol: &str, sentiment_id: &str) -> String {
        format!("{}:{}", symbol, sentiment_id)
    }
}

// Implement VectorEmbedding for semantic search
impl VectorEmbedding for TickerEmbedding {
    fn vector(&self) -> &[f32] {
        &self.vector
    }
}
