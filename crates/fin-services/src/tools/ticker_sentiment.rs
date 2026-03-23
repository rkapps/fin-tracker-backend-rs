use std::{collections::HashMap, sync::Arc};

use agentic_core::client::{embeddings::Embedding, tools::Tool};
use anyhow::Result;
use async_trait::async_trait;
use fin_domain::ticker::TickerEmbedding;
use fin_storage::service::StorageService;
use serde_json::{Value, json};
use storage_core::vector::search;
use tracing::debug;

use crate::tools::TickerParam;

#[derive(Debug)]
pub struct TickerSentimentTool {
    query_embedding: Embedding,
    storage_service: Arc<dyn StorageService>,
}
impl TickerSentimentTool {
    pub fn new(
        query_embedding: Embedding,
        storage_service: Arc<dyn StorageService>,
    ) -> TickerSentimentTool {
        Self {
            query_embedding,
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
                "symbol": {
                    "type": "string",
                    "description": "Ticker symbol"
                }
            },
            "required": ["symbol"]
        })
    }

    async fn execute(&self, value: serde_json::Value) -> Result<Value> {
        let ticker_param: TickerParam = match serde_json::from_value(value.clone()) {
            Ok(c) => c,
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Error dezerializing arguments: {:#?} - {:?}",
                    value,
                    e
                ));
            }
        };

        //Get ticker
        // let ticker = self.storage_service.get_ticker(&ticker_param.symbol).await?;
        debug!("Ticker sentiment params {:#?}", ticker_param.symbol);

        let embeddings = self
            .storage_service
            .get_ticker_embeddings(&ticker_param.symbol)
            .await?;
        // debug!("Embeddings: {}", embeddings.len());
        let idsm: HashMap<String, TickerEmbedding> = embeddings
            .iter()
            .map(|embedding| (embedding.id.clone(), embedding.clone()))
            .collect();

        //build the candidates from the embeddings for apple stock
        let candidates: Vec<(String, Vec<f32>)> = embeddings
            .into_iter()
            .map(|entry| (entry.id.clone(), entry.vector.clone()))
            .collect();

        // top 5 similarit results from vector_search
        let vectors = self.query_embedding.clone().into_vec();
        let results = search::<String>(&vectors, &candidates, 5);

        // iterator through result and return vector of (TickerEmbedding, f32)
        let final_results: Vec<(TickerEmbedding, f32)> = results
            .iter()
            .filter_map(|(id, score)| idsm.get(id).cloned().map(|item| (item, *score)))
            .collect();

        let texts = final_results
            .iter()
            .map(|entry| {
                let text = entry.0.embedding_text.as_str();
                text.chars().take(150).collect::<String>()
            })
            .collect::<Vec<String>>()
            .join(", ");

        debug!("sentiments for symbol: {} - {}", ticker_param.symbol, texts);

        // Format the the summarize contents to the llm
        let content: String = format!("{{ {} }}", texts);
        Ok(json!({
            "symbol": ticker_param.symbol,
            "sentiment_count": texts.len(),
            "sentiments": content
        }))
    }
}
