use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use fin_storage::service::StorageService;
use rustic_agent::client::tools::Tool;
use rustic_ml::{embeddings::client::Embedding, search::similarity::search};
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::info;

#[derive(Debug)]
pub struct TickerSimilarityTool {
    query_embedding: Embedding,
    storage_service: Arc<dyn StorageService>,
}
impl TickerSimilarityTool {
    pub fn new(
        query_embedding: Embedding,
        storage_service: Arc<dyn StorageService>,
    ) -> TickerSimilarityTool {
        Self {
            query_embedding,
            storage_service,
        }
    }
}

#[async_trait]
impl Tool for TickerSimilarityTool {
    fn name(&self) -> String {
        "ticker_similarity".to_string()
    }

    fn description(&self) -> String {
        "Finds stocks similar to a query using semantic vector search. \
     ALWAYS use this tool when the user asks to compare or find stocks by theme or category. \
     Examples that MUST use this tool: \
     'software infrastructure stocks', \
     'cloud security companies', \
     'healthcare medical device stocks', \
     'find companies similar to CrowdStrike'. \
     Only skip this tool for named ETF families like 'SPDR ETFs' or 'FAANG stocks' \
     where the tickers are universally known. \
     Only analyse tickers returned by this tool — never add from training knowledge."
            .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "limit": {
                    "type": "integer",
                    "description": "Max number of similar stocks to return. Defaults to 5."
                }
            }
        })
    }

    async fn execute(&self, value: serde_json::Value) -> Result<Value> {
        #[derive(Debug, Deserialize)]
        struct Params {
            #[serde(default = "default_limit")]
            limit: usize,
        }
        fn default_limit() -> usize {
            5
        }

        let params: Params = serde_json::from_value(value.clone())
            .map_err(|e| anyhow::anyhow!("Failed to deserialize params: {:?} — {:?}", value, e))?;

        let overview_candidates = self
            .storage_service
            .get_ticker_overview_embeddings()
            .await?;

        let industry_candidates = self
            .storage_service
            .get_ticker_industry_embeddings()
            .await?;

        // Score both
        let vectors = self.query_embedding.clone().into_vec();
        let overview_scores: HashMap<String, f32> = search(
            &vectors,
            &overview_candidates
                .iter()
                .map(|(t, e)| (t.symbol.clone(), e.clone()))
                .collect::<Vec<_>>(),
            50,
        )
        .into_iter()
        .collect();

        let industry_scores: HashMap<String, f32> = search(
            &vectors,
            &industry_candidates
                .iter()
                .map(|(t, e)| (t.symbol.clone(), e.clone()))
                .collect::<Vec<_>>(),
            50,
        )
        .into_iter()
        .collect();

        // Combine scores
        let mut combined: Vec<(String, f32)> = overview_scores
            .iter()
            .map(|(symbol, ov_score)| {
                let ind_score = industry_scores.get(symbol).unwrap_or(&0.0);
                let combined = ov_score * 0.5 + ind_score * 0.5;
                (symbol.clone(), combined)
            })
            .collect();

        combined.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let symbols: Vec<String> = combined
            .into_iter()
            .take(params.limit)
            .map(|(s, _)| s)
            .collect();

        info!("Similar_stocks: {:?}", symbols);
        Ok(json!({ "similar_tickers": symbols }))
    }
}
