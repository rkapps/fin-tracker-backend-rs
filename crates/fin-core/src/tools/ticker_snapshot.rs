use std::sync::Arc;

use agentic_core::client::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;
use fin_domain::dto::ticker_param::TickerParam;
use fin_storage::service::StorageService;
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
        info!("Ticker Snapshot params {:#?}", ticker_param.symbol);
        let ticker = match self
            .storage_service
            .get_ticker_by_symbol(&ticker_param.symbol)
            .await
        {
            Ok(t) => t,
            Err(_) => {
                return Ok(json!({
                    "symbol": ticker_param.symbol,
                    "error": "Ticker not found in database"
                }));
            }
        };

        // In get_ticker_snapshot execute()
        let mut fundamentals = json!({});
        if let Some(mc) = ticker.market_cap {
            fundamentals["market_cap"] = json!(mc);
        }
        if ticker.r#yield > 0.0 {
            fundamentals["yield"] = json!(ticker.r#yield);
        }
        if let Some(eps) = ticker.eps {
            fundamentals["eps"] = json!(eps);
        }
        if let Some(pe) = ticker.pe_ratio {
            fundamentals["pe_ratio"] = json!(pe);
        }
        if let Some(fpe) = ticker.forward_pe {
            fundamentals["forward_pe"] = json!(fpe);
        }
        if let Some(peg) = ticker.peg_ratio {
            fundamentals["peg_ratio"] = json!(peg);
        }
        if let Some(pb) = ticker.pb_ratio {
            fundamentals["pb_ratio"] = json!(pb);
        }
        if let Some(ps) = ticker.ps_ratio {
            fundamentals["ps_ratio"] = json!(ps);
        }
        if let Some(beta) = ticker.beta {
            fundamentals["beta"] = json!(beta);
        }
        if let Some(consensus) = ticker.analyst_consensus {
            fundamentals["analyst_consensum"] = json!(consensus);
        }

        if let Some(target) = ticker.analyst_target_price {
            fundamentals["analyst_target_price"] = json!(target);
        }

        // etc
        let snapshot = json!({
            "symbol": ticker.symbol,
            "name": ticker.name,
            "sector": ticker.sector,
            "industry": ticker.industry,
            "price": {
                "last": ticker.pr_last.to_string(),
                "prev": ticker.pr_prev.to_string(),
                "open": ticker.pr_open.to_string(),
                "high": ticker.pr_high.to_string(),
                "low": ticker.pr_low.to_string(),
                "change_amt": ticker.pr_diff_amt,
                "change_perc": ticker.pr_diff_perc,
                "52wk_high": ticker.pr_52_wk_high,
                "52wk_low": ticker.pr_52_wk_low,
            },
            "fundamentals": fundamentals,
            "performance" : ticker.performance,
            "signals": ticker.signals
        });

        debug!("Snapshot: {:#?}", snapshot);

        Ok(snapshot)
    }
}
