use std::sync::Arc;

use agentic_core::capabilities::client::tool::Tool;
use anyhow::Result;
use async_trait::async_trait;
use fin_storage::service::StorageService;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::debug;

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
                "symbol": {
                    "type": "string",
                    "description": "The stock ticker symbol"
                },
                "indicators": {
                    "type": "array",
                    "items": { "type": "string", "enum": ["RSI", "MACD", "SMA", "EMA", "BB"] },
                    "description": "List of indicators to return. Omit to return all."
                }
            },
            "required": ["symbol"]
        })
    }

    async fn execute(&self, value: serde_json::Value) -> Result<Value> {
        #[derive(Debug, Deserialize)]
        struct Params {
            symbol: String,
            _indicators: Option<Vec<String>>,
        }
        let params: Params = serde_json::from_value(value.clone())
            .map_err(|e| anyhow::anyhow!("Failed to deserialize params: {:?} — {:?}", value, e))?;
        debug!("Ticker indicator params {:#?}", params);

        let ticker = match self.storage_service.get_ticker(&params.symbol).await {
            Ok(t) => t,
            Err(_) => {
                return Ok(json!({
                    "symbol": params.symbol,
                    "error": "Ticker not found in database"
                }));
            }
        };

        debug!("Ticker technical indicators params {:#?}", params);
        let latest_indicators = self
            .storage_service
            .get_ticker_indicators_latest(&ticker.symbol)
            .await?;
        debug!("Indicators: {:#?}", latest_indicators);
        Ok(json!({
            "symbol": params.symbol,
            "indicators": latest_indicators[0]
        }))
    }
}
