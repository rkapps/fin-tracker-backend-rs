use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use fin_storage::service::StorageService;
use rustic_agent::Tool;
use rustic_ml::search;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::{debug, info};

#[derive(Debug)]
pub struct TickerPeersTool {
    storage_service: Arc<dyn StorageService>,
}
impl TickerPeersTool {
    pub fn new(storage_service: Arc<dyn StorageService>) -> TickerPeersTool {
        Self { storage_service }
    }
}

#[async_trait]
impl Tool for TickerPeersTool {
    fn name(&self) -> String {
        "ticker_peers".to_string()
    }

    fn description(&self) -> String {
        "Returns peer stocks for a given ticker across three dimensions: \
 industry peers (same industry), sector peers (same sector), \
 and similar stocks (pre-computed embedding similarity on business description). \
 ALWAYS call this tool first before fetching any peer data. \
 Never use training knowledge to assume peers. \
 Use the returned symbols to decide which stocks to analyse further."
            .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "symbol": {
                    "type": "string",
                    "description": "The stock ticker symbol"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max number of peers to return.",
                    "default": 10
                }
            },
            "required": ["symbol", "limit"]
        })
    }

    async fn execute(&self, value: serde_json::Value) -> Result<Value> {
        #[derive(Debug, Deserialize)]
        struct Params {
            symbol: String,
            #[serde(default = "default_limit")]
            limit: usize,
        }

        fn default_limit() -> usize {
            10
        }

        let params: Params = serde_json::from_value(value.clone())
            .map_err(|e| anyhow::anyhow!("Failed to deserialize params: {:?} — {:?}", value, e))?;

        info!("Ticker Peers params {:#?}", params);
        let ticker = match self
            .storage_service
            .get_ticker_by_symbol(&params.symbol)
            .await
        {
            Ok(t) => t,
            Err(_) => {
                return Ok(json!({
                    "symbol": params.symbol,
                    "error": "Ticker not found in database"
                }));
            }
        };

        let mut industry_peers: Vec<String> = Vec::new();
        let tickers = self
            .storage_service
            .get_ticker_peers_by_industry(&params.symbol)
            .await?;
        debug!("Industry tickers all: {}", tickers.len());

        let mut count = 0;
        for ticker in tickers {
            if count > params.limit {
                break;
            }
            if ticker.symbol == params.symbol {
                continue;
            }
            count += 1;
            industry_peers.push(ticker.symbol);
        }

        let mut sector_peers: Vec<String> = Vec::new();

        if industry_peers.len() < params.limit {
            let sector_tickers = self
                .storage_service
                .get_ticker_peers_by_sector(&params.symbol)
                .await?;
            debug!("Sector peers total: {}", sector_tickers.len());

            // Get ticker embedding
            let ticker_embedding = ticker.overview_embedding.clone().unwrap_or_default();

            // Build candidates from sector tickers
            let candidates: Vec<(String, Vec<f32>)> = sector_tickers
                .iter()
                .filter(|t| t.symbol != params.symbol)
                .filter_map(|t| {
                    t.overview_embedding
                        .clone()
                        .map(|emb| (t.symbol.clone(), emb))
                })
                .collect();

            //add a larger limit
            let ranked = search(&ticker_embedding, &candidates, 20);
            for (symbol, score) in &ranked {
                debug!("Sector peer score: {}-{}", symbol, score);
            }
            sector_peers = ranked
                .iter()
                .filter(|(_, score)| *score > 0.45) // threshold — tune this
                .take(params.limit)
                .map(|(symbol, _)| symbol.clone())
                .collect();
            // sector_peers = ranked.iter().map(|(symbol, _)| symbol.clone()).collect();
        }

        info!("Industry peers {:#?}", industry_peers);
        info!("sector peers {:#?}", sector_peers);
        Ok(json!({
            "symbol": params.symbol,
            "industry_peers": industry_peers,
            "sector_peers": sector_peers
        }))
    }
}
