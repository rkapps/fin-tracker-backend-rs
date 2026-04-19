use crate::tickers::deserialize_flexible_datetime;
use crate::tickers::serialize_as_bson_datetime;
use crate::utils::string_utils::string_to_float;
use chrono::{DateTime, Utc};
use fin_providers::alpha::model::AlphaTickerSentimentFeed;
use serde::{Deserialize, Serialize};
use storage_core::core::RepoModel;

use crate::{
    tickers::TICKER_SENTIMENT_COLLECTION_NAME, utils::string_utils::alpha_string_to_utc_datetime,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TickerSentiment {
    pub id: String,
    pub symbol: String,

    #[serde(
        deserialize_with = "deserialize_flexible_datetime",
        serialize_with = "serialize_as_bson_datetime"
    )]
    pub date: DateTime<Utc>,
    pub title: String,
    pub url: String,
    pub summary: String,
    pub source: String,
    pub source_category: String,
    pub source_domain: String,
    pub relevance_score: f64,
    pub score: f64,
    pub label: String,
}

impl RepoModel<String> for TickerSentiment {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn collection(&self) -> &'static str {
        TICKER_SENTIMENT_COLLECTION_NAME
    }
}

impl TickerSentiment {
    pub fn new_from_alpha_feed(
        symbol: &str,
        feed: &AlphaTickerSentimentFeed,
    ) -> Option<TickerSentiment> {
        if let Some(sentiment) = feed
            .ticker_sentiment
            .iter()
            .find(|f| f.ticker.to_uppercase() == symbol.to_uppercase())
        {
            let date_published = alpha_string_to_utc_datetime(&feed.time_published);
            Some(TickerSentiment {
                id: TickerSentiment::sentiment_id(symbol, feed),
                date: date_published,
                label: sentiment.ticker_sentiment_label.clone(),
                relevance_score: string_to_float(&sentiment.relevance_score),
                score: string_to_float(&sentiment.ticker_sentiment_score),
                source: feed.source.to_string(),
                source_category: feed.category_within_source.to_string(),
                source_domain: feed.source_domain.to_string(),
                summary: feed.summary.clone().unwrap_or_default(),
                symbol: symbol.to_string(),
                title: feed.title.to_string(),
                url: feed.url.to_string(),
            })
        } else {
            None
        }
    }
    fn sentiment_id(symbol: &str, feed: &AlphaTickerSentimentFeed) -> String {
        format!("{}:{}", symbol, feed.time_published)
    }

    pub fn embedding_text(&self) -> String {
        format!(
            "Stock: {}. Sentiment: {} ({}). {}. {}",
            self.symbol, self.label, self.score, self.title, self.summary
        )
    }

    // Convert batch, filtering out any feeds that fail to parse
    pub fn new_from_alpha_batch(
        symbol: &str,
        feeds: Vec<AlphaTickerSentimentFeed>,
    ) -> Vec<TickerSentiment> {
        feeds
            .iter()
            .filter_map(|f| TickerSentiment::new_from_alpha_feed(symbol, f))
            .collect()
    }
}
