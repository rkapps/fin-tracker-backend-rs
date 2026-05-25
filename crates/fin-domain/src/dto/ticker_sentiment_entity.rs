use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct TickerSentimentEntity {
    pub id: String,
    pub date: DateTime<Utc>,
    pub symbol: String,
    pub title: String,
    pub score: f64,
    pub label: String,
    pub source: String,
    pub similarity: f32, // from semantic_search result
}
