use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use fin_storage::service::StorageService;
use rustic_agent::client::tools::Tool;
use serde_json::{Value, json};
use tracing::{debug, info};

#[derive(Debug)]
pub struct TickerTaxonomyTool {
    storage_service: Arc<dyn StorageService>,
}
impl TickerTaxonomyTool {
    pub fn new(storage_service: Arc<dyn StorageService>) -> TickerTaxonomyTool {
        Self { storage_service }
    }
}

#[async_trait]
impl Tool for TickerTaxonomyTool {
    fn name(&self) -> String {
        "ticker_taxonomy".to_string()
    }

    fn description(&self) -> String {
        "Returns all available sectors and their industries from the database. \
         ALWAYS call this tool first before calling ticker_screening when the user \
         asks to find or compare stocks by sector, industry or theme. \
         Use the returned values to populate the industry parameter in ticker_screening exactly."
            .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {},
        })
    }

    async fn execute(&self, _value: serde_json::Value) -> Result<Value> {
        info!("Ticker taxonomy");
        let groups = self.storage_service.get_ticker_groups().await?;
        debug!("Ticker groups: {:?}", groups);
        Ok(json!(groups))
    }
}
