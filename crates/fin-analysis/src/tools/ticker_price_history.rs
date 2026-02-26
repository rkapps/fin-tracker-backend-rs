use std::sync::Arc;

use agentic_core::capabilities::client::tool::Tool;
use anyhow::Result;
use async_trait::async_trait;
use fin_domain::utils::string_utils::string_to_utc_datetime;
use fin_storage::service::StorageService;
use serde::Deserialize;
use serde_json::{Value, json};
use tracing::info;

#[derive(Debug)]
pub struct TickerPriceHistoryTool {
    storage_service: Arc<dyn StorageService>,
}
impl TickerPriceHistoryTool {
    pub fn new(storage_service: Arc<dyn StorageService>) -> TickerPriceHistoryTool {
        Self { storage_service }
    }
}

#[async_trait]
impl Tool for TickerPriceHistoryTool {
    fn name(&self) -> String {
        "ticker_price_history".to_string()
    }

    fn description(&self) -> String {
        "Returns daily price history for a stock ticker. \
 Use this when the user asks for price action or daily trading history. \
 Map user requests to periods: 'last week' = 7 days, 'last month' = 30 days, \
 'last 3 months' = 90 days. \
 Do not use this for period returns or performance — \
 use get_ticker_snapshot which has pre-computed performance data."
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
                    "from": {
                        "type": "string",
                        "description": "Start date in YYYY-MM-DD format"
                    },
                    "to": {
                        "type": "string",
                        "description": "End date in YYYY-MM-DD format. Defaults to today."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max number of trading days to return. Defaults to 30.",
                        "default": 30
                    }
                },
                "required": ["symbol", "from"]
        })
    }

    async fn execute(&self, value: serde_json::Value) -> Result<Value> {
        #[derive(Debug, Deserialize)]
        struct Params {
            symbol: String,
            from: String,
            // to: Option<String>,
            #[serde(default = "default_limit")]
            limit: usize,
        }
        fn default_limit() -> usize {
            30
        }

        let params: Params = serde_json::from_value(value.clone())
            .map_err(|e| anyhow::anyhow!("Failed to deserialize params: {:?} — {:?}", value, e))?;

        info!("Ticker History params {:#?}", params);
        let Some(from_date) = string_to_utc_datetime(&params.from) else {
            return Err(anyhow::anyhow!("Error converting from date"));
        };

        let eod_data = self
            .storage_service
            .get_ticker_history_by_date(&params.symbol, from_date)
            .await?;

        let prices: Vec<Value> = eod_data
            .into_iter()
            .take(params.limit)
            .map(|d| {
                json!({
                    "date": d.date,
                    "open": d.open,
                    "high": d.high,
                    "low": d.low,
                    "close": d.close,
                    "volume": d.volume
                })
            })
            .collect();

        Ok(json!({
            "symbol": params.symbol,
            "from": params.from,
            "count": prices.len(),
            "prices": prices
        }))
    }
}
