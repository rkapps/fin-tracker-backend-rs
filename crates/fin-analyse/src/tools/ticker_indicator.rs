use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use fin_domain::dto::ticker_search_param::TickerSearchParam;
use fin_storage::{mongo::news, service::StorageService};
use rustic_core::Tool;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::{debug, error, info};

#[derive(Debug)]
pub struct TickerIndicatorTool {
    storage_service: Arc<dyn StorageService>,
}
impl TickerIndicatorTool {
    pub fn new(storage_service: Arc<dyn StorageService>) -> TickerIndicatorTool {
        Self { storage_service }
    }
}

#[async_trait]
impl Tool for TickerIndicatorTool {
    fn name(&self) -> String {
        "ticker_indicator".to_string()
    }

    fn description(&self) -> String {
        "Returns technical indicators (RSI, MACD, moving averages, Bollinger Bands) for a stock ticker. \
     ALWAYS call this tool for every ticker being analysed, without exception. \
     Never skip this tool — it is required for MACD, RSI, and Bollinger Band rows in the response. \
     If this tool is not called, those rows must show 'Data unavailable' which is unacceptable."
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
                "indicators": {
                    "type": "array",
                    "items": { "type": "string", "enum": ["RSI", "MACD", "SMA", "EMA", "BB"] },
                    "description": "List of indicators to return. Omit to return all."
                }
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

        info!("Ticker indicators symbols {:?}", params.symbols);
        let indicators = self
            .storage_service
            .get_ticker_indicators_by_symbols(params.symbols, Some(1))
            .await?;
        let elapsed = start.elapsed();
        info!("Indicators: {:?}  {:.1}s", indicators.len(), elapsed.as_secs_f32());

        Ok(json!({ "indicators": indicators }))

    }
}
