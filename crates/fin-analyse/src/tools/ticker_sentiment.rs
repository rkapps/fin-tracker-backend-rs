use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use fin_domain::tickers::TickerEmbedding;
use fin_storage::service::StorageService;
use rustic_agent::Tool;
use rustic_ml::{EmbeddingClient, search};
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
                "symbol": {
                    "type": "string",
                    "description": "Ticker symbol"
                },
                "query": {
                "type": "string",
                "description": "The user's query or context used to find relevant sentiment. e.g. 'AI chip demand', 'earnings outlook', 'competitive positioning'"
            }
            },
            "required": ["symbol", "query"]
        })
    }

    async fn execute(&self, params: serde_json::Value) -> Result<Value> {
        let symbol = params["symbol"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("symbol required"))?;

        let query = params["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("query required"))?;

        // embed the query at call time
        let query_embedding = self
            .embedding_client
            .embed_text(&format!("query: {}", query))
            .await?;

        //Get ticker
        // let ticker = self.storage_service.get_ticker(&ticker_param.symbol).await?;
        info!("Ticker sentiment query {:#?} symbol {}", query, symbol);

        let embeddings = self.storage_service.get_ticker_embeddings(symbol).await?;
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
        let vectors = query_embedding.clone().into_vec();
        let results = search::<String>(&vectors, &candidates, 5);

        // iterator through result and return vector of (TickerEmbedding, f32)
        // let final_results: Vec<(TickerEmbedding, f32)> = results
        //     .iter()
        //     .filter_map(|(id, score)| idsm.get(id).cloned().map(|item| (item, *score)))
        //     .collect();

        // let texts = final_results
        //     .iter()
        //     .map(|entry| {
        //         let text = entry.0.embedding_text.as_str();
        //         text.chars().take(150).collect::<String>()
        //     })
        //     .collect::<Vec<String>>()
        //     .join(", ");
        // info!("results: {:?}", texts);

        // debug!("sentiments for symbol: {} - {}", ticker_param.symbol, texts);

        // // Format the the summarize contents to the llm
        // let content: String = format!("{{ {} }}", texts);
        // Ok(json!({
        //     "symbol": ticker_param.symbol,
        //     "sentiment_count": texts.len(),
        //     "sentiments": content
        // }))

        let final_results: Vec<(TickerEmbedding, f32)> = results
            .iter()
            .filter_map(|(id, score)| idsm.get(id).cloned().map(|item| (item, *score)))
            .collect();
        let sentiments: Vec<serde_json::Value> = final_results
            .iter()
            .map(|entry| {
                json!({
                    "text": entry.0.embedding_text.as_str().chars().take(150).collect::<String>(),
                    // "score": entry.0.e,
                    // "label": entry.0.label,
                })
            })
            .collect();
        debug!("results: {:?}", sentiments);

        Ok(json!({
            "symbol": symbol,
            "sentiment_count": final_results.len(),
            "sentiments": sentiments
        }))
    }
}
