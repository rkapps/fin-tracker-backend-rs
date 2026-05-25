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

        // // In get_ticker_snapshot execute()
        // let mut fundamentals = json!({});
        // if let Some(mc) = ticker.total_assets {
        //     fundamentals["total_assets"] = json!(mc);
        // }
        // if ticker.r#yield > 0.0 {
        //     fundamentals["yield"] = json!(ticker.r#yield);
        // }
        // if let Some(eps) = ticker.eps {
        //     fundamentals["eps"] = json!(eps);
        // }
        // if let Some(pe) = ticker.pe_ratio {
        //     fundamentals["pe_ratio"] = json!(pe);
        // }
        // if let Some(fpe) = ticker.forward_pe {
        //     fundamentals["forward_pe"] = json!(fpe);
        // }
        // if let Some(peg) = ticker.peg_ratio {
        //     fundamentals["peg_ratio"] = json!(peg);
        // }
        // if let Some(pb) = ticker.pb_ratio {
        //     fundamentals["pb_ratio"] = json!(pb);
        // }
        // if let Some(ps) = ticker.ps_ratio {
        //     fundamentals["ps_ratio"] = json!(ps);
        // }
        // if let Some(beta) = ticker.beta {
        //     fundamentals["beta"] = json!(beta);
        // }
        // if let Some(consensus) = ticker.analyst_consensus {
        //     fundamentals["analyst_consensum"] = json!(consensus);
        // }

        // if let Some(target) = ticker.analyst_target_price {
        //     fundamentals["analyst_target_price"] = json!(target);
        // }

        // // etc
        // let snapshot = json!({
        //     "symbol": ticker.symbol,
        //     "name": ticker.name,
        //     "sector": ticker.sector,
        //     "industry": ticker.industry,
        //     "price": {
        //         "last": ticker.pr_last.to_string(),
        //         "prev": ticker.pr_prev.to_string(),
        //         "open": ticker.pr_open.to_string(),
        //         "high": ticker.pr_high.to_string(),
        //         "low": ticker.pr_low.to_string(),
        //         "change_amt": ticker.pr_diff_amt,
        //         "change_perc": ticker.pr_diff_perc,
        //         "52wk_high": ticker.pr_52_wk_high,
        //         "52wk_low": ticker.pr_52_wk_low,
        //     },
        //     "fundamentals": fundamentals,
        //     "performance" : ticker.performance,
        //     "signals": ticker.signals
        // });

        let snapshots: Vec<TickerSnapshot> =
            tickers.into_iter().map(TickerSnapshot::from).collect();

        debug!("Snapshot: {:#?}", snapshots);
        Ok(serde_json::to_value(snapshots)?)
    }
}
