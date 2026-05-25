use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use fin_domain::dto::ticker_snapshot::TickerSnapshot;
use fin_storage::service::StorageService;
use rustic_core::Tool;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::{debug, info};

#[derive(Debug)]
pub struct TickerSnapshotTool {
    storage_service: Arc<dyn StorageService>,
}
impl TickerSnapshotTool {
    pub fn new(storage_service: Arc<dyn StorageService>) -> TickerSnapshotTool {
        Self { storage_service }
    }
}

#[async_trait]
impl Tool for TickerSnapshotTool {
    fn name(&self) -> String {
        "ticker_snapshot".to_string()
    }

    fn description(&self) -> String {
        "Returns the current state of a stock ticker including latest price, \
 52-week range, fundamentals (PE, EPS, market cap, PEG, PB, PS ratios), \
 dividend info, and recent performance. Use this for valuation and current price context."
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
            },
            "required": ["symbols"]
        })
    }

    async fn execute(&self, value: serde_json::Value) -> Result<Value> {
        #[derive(Debug, Deserialize)]
        struct Params {
            symbols: Vec<String>,
        }
        let start = std::time::Instant::now();
        let params: Params = serde_json::from_value(value.clone())
            .map_err(|e| anyhow::anyhow!("Failed to deserialize params: {:?} — {:?}", value, e))?;

        info!("Ticker Snapshot params {:#?}", params.symbols);
        let tickers = match self
            .storage_service
            .get_tickers_by_symbols(params.symbols.clone())
            .await
        {
            Ok(t) => t,
            Err(_) => {
                return Ok(json!({
                    "symbol": params.symbols,
                    "error": "Ticker not found in database"
                }));
            }
        };


        let snapshots: Vec<TickerSnapshot> =
            tickers.into_iter().map(TickerSnapshot::from).collect();

        debug!("Snapshot: {:#?}", snapshots);

        let elapsed = start.elapsed();
        info!("Snapshots: {:?}  {:.1}s", snapshots.len(), elapsed.as_secs_f32());
        Ok(json!({"snapshots": snapshots }))

    }
}
