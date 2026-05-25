use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use fin_core::tickers::sentiments::search_ticker_sentiments;
use fin_domain::{dto::ticker_search_param::TickerSearchParam, tickers::TickerEmbedding};
use fin_storage::service::StorageService;
use rustic_core::Tool;
use rustic_ml::{EmbeddingClient, search};
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::{debug, info};

#[derive(Debug)]
pub struct TickerSentimentTool {
    embedding_client: Arc<dyn EmbeddingClient>,
    storage_service: Arc<dyn StorageService>,
}
impl TickerSentimentTool {
    pub fn new(
        embedding_client: Arc<dyn EmbeddingClient>,
        storage_service: Arc<dyn StorageService>,
    ) -> TickerSentimentTool {
        Self {
            embedding_client,
            storage_service,
        }
    }
}

#[async_trait]
impl Tool for TickerSentimentTool {
    fn name(&self) -> String {
        "ticker_sentiment".to_string()
    }

    fn description(&self) -> String {
        "Returns relevant sentiment analysis and news for a stock ticker, \
 filtered by the user's query context. Use this to understand market \
 narrative, news-driven momentum, and investor sentiment."
            .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "symbols": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of stock ticker symbols to find peers for"
                },
                "query": {
                "type": "string",
                "description": "The user's query or context used to find relevant sentiment. e.g. 'AI chip demand', 'earnings outlook', 'competitive positioning'"
            }
            },
            "required": ["symbols", "query"]
        })
    }

    async fn execute(&self, value: serde_json::Value) -> Result<Value> {
        #[derive(Debug, Deserialize)]
        struct Params {
            symbols: Vec<String>,
            query: String,
            #[serde(default = "default_limit")]
            limit: usize,
        }
        fn default_limit() -> usize {
            10
        }
        let params: Params = serde_json::from_value(value.clone())
            .map_err(|e| anyhow::anyhow!("Failed to deserialize params: {:?} — {:?}", value, e))?;

        info!(
            "Ticker sentiment symbols {:?} query {:#?} ",
            params.symbols, params.query
        );

        let sentiments = search_ticker_sentiments(
            self.storage_service.clone(),
            self.embedding_client.clone(),
            params.symbols,
            params.query,
            params.limit,
        )
        .await?;

        let json = serde_json::to_value(
            sentiments
                .iter()
                .map(|s| {
                    json!({
                        "symbol": s.symbol,
                        "date": s.date.format("%Y-%m-%d").to_string(),
                        "title": s.title,
                        "score": (s.score * 100.0).round() / 100.0,
                        "label": s.label,
                        "source": s.source,
                    })
                })
                .collect::<Vec<_>>(),
        )?;

        Ok(json)
    }
}
