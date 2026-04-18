use agentic_core::client::tools::Tool;
use anyhow::Result;
use async_trait::async_trait;
use fin_domain::dto::screen_param::TickerScreenParam;
use serde_json::{Value, json};
use std::sync::Arc;
use tracing::info;

use crate::stocks::StocksService;

#[derive(Debug)]
pub struct TickerScreeningTool {
    stocks_service: Arc<StocksService>,
}
impl TickerScreeningTool {
    pub fn new(stocks_service: Arc<StocksService>) -> TickerScreeningTool {
        Self { stocks_service }
    }
}

#[async_trait]
impl Tool for TickerScreeningTool {
    fn name(&self) -> String {
        "ticker_screening".to_string()
    }

    fn description(&self) -> String {
        "Finds and filters stocks or ETFs based on any combination of semantic query, technical signals, industry, market cap, and asset type. \
            ALWAYS use this tool when the user asks to find, compare, or screen stocks by theme, category, or condition. \
            Do not use ticker_peers for theme or category queries — use this tool instead. \
        \
            Always call ticker_taxonomy before this tool when the query mentions a specific industry or sector. \
        \
            Query guidelines: \
                - Pass the user's original query text unchanged. Never replace with generic terms like 'find stocks'. \
                - Only include assets_cap_range if the user explicitly mentions a assets cap size. Never infer it. \
                - When the query implies a specific industry, populate both query and industry fields. \
        \
        Examples: \
            'software infrastructure mid cap stocks' → query: 'software infrastructure', industry: 'Software - Infrastructure', assets_cap_range: 'mid' \
            'find oversold cloud security companies' → query: 'cloud security', signals: ['RSI Oversold'] \
            'mostly oversold' or 'heavily oversold'  → signals: ['Deeply Oversold']\
            'defensive buy rated stocks' → signals: ['Low Beta', 'Analyst Buy'] \
            'compare spider ETFs' → query: 'SPDR ETFs', asset_type: 'etf' \
        \
        Signal rules: \
            Pass ONE signal per category. Results must match ALL signals provided. \
            Categories: \
                Trend: 'Golden Cross', 'Death Cross', 'Above SMA50', 'Below SMA50'. \
                Momentum: 'MACD Bullish Crossover', 'MACD Bearish Crossover'. \
                RSI: 'RSI Oversold', 'RSI Overbought', 'Deeply Oversold', 'Mostly Oversold' \
                Bands: 'BB Breakout Upper', 'BB Breakout Lower', 'BB Squeeze'. \
                Stochastic: 'Stochastic Bullish', 'Stochastic Bearish'. \
                Analyst: 'Analyst Strong Buy', 'Analyst Buy', 'Analyst Hold', 'Analyst Sell', 'Analyst Strong Sell'. \
                Beta: 'Low Beta', 'Market Beta', 'High Beta', 'Very High Beta'."        
                .to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Semantic search query for business theme or category. Example: 'software infrastructure', 'cloud security', 'payments processing'."
                },
                "signals": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": [
                            "SMA Stack Bullish", "SMA Stack Bearish",
                            "Bullish Pullback", "Bearish Rally",
                            "Above SMA50", "Below SMA50",
                            "Golden Cross", "Death Cross",
                            "MACD Bullish Crossover", "MACD Bearish Crossover",
                            "MACD Histogram Expanding", "MACD Histogram Weakening",
                            "BB Breakout Upper", "BB Breakout Lower", "BB Squeeze",
                            "Stochastic Bullish", "Stochastic Bearish",
                            "RSI Oversold", "RSI Recovering from Oversold", "RSI Multi-period Oversold",
                            "RSI Overbought", "RSI Recovering from Overbought", "RSI Multi-period Overbought",
                            "Oversold Confluence", "Oversold Reversal Setup",
                            "Overbought Confluence", "Overbought Reversal Setup",
                            "Deeply Oversold", "Moderately Oversold",
                            "Mean Reversion Candidate",
                            "Momentum Breakout", "Bullish Trend Exhaustion", "Bearish Trend Exhaustion",
                            "Volatility Expanding", "Volatility Contracting",
                            "Analyst Strong Buy", "Analyst Buy", "Analyst Hold",
                            "Analyst Sell", "Analyst Strong Sell",
                            "Low Beta", "Market Beta", "High Beta", "Very High Beta",

                            // --- ML cross-period summary ---
                            "ML Strong Bull — All Periods All Models Confirmed",
                            "ML Strong Bull — All Periods Confirmed",
                            "ML Strong Bull — All Periods LR+MLP Confirmed",
                            "ML Strong Bear — All Periods All Models Confirmed",
                            "ML Strong Bear — All Periods Confirmed",
                            "ML Strong Bear — All Periods LR+MLP Confirmed",

                            // --- ML per-period confluence ---
                            "ML5 Bullish — 2/2 Confirmed",  "ML5 Bullish — 3/3 Confirmed",
                            "ML5 Bearish — 2/2 Confirmed",  "ML5 Bearish — 3/3 Confirmed",
                            "ML10 Bullish — 2/2 Confirmed", "ML10 Bullish — 3/3 Confirmed",
                            "ML10 Bearish — 2/2 Confirmed", "ML10 Bearish — 3/3 Confirmed",
                            "ML20 Bullish — 2/2 Confirmed", "ML20 Bullish — 3/3 Confirmed",
                            "ML20 Bearish — 2/2 Confirmed", "ML20 Bearish — 3/3 Confirmed",
                            "ML60 Bullish — 2/2 Confirmed", "ML60 Bullish — 3/3 Confirmed",
                            "ML60 Bearish — 2/2 Confirmed", "ML60 Bearish — 3/3 Confirmed",

                            // --- MLP per-period ---
                            "MLP5 Bullish",  "MLP5 Bearish",
                            "MLP10 Bullish", "MLP10 Bearish",
                            "MLP20 Bullish", "MLP20 Bearish",
                            "MLP60 Bullish", "MLP60 Bearish",
                        ]
                    },
                    "description": "Filter by active technical or analyst signals."
                },
                "industry": {
                    "type": "string",
                    "description": "Filter by industry, partial match. Example: 'semiconductors', 'medical devices'."
                },
                "assets_cap_range": {
                    "type": "string",
                    "enum": ["mega", "large", "mid", "small"],
                    "description": "Filter by assets cap. mega: >$1T, large: $100B-$1T, mid: $2B-$100B, small: <$2B. \
                            Map: 'mid cap' → 'mid', 'large cap' → 'large', 'small cap' → 'small', 'mega cap' → 'mega'."
                },
                "asset_type": {
                    "type": "string",
                    "enum": ["stock", "etf"],
                    "description": "Filter by asset type. Defaults to 'stock'."
                },
                "limit": {
                    "type": "integer",
                    "description": "Max number of results to return. Defaults to 10."
                }
            }
        })
    }

    async fn execute(&self, value: serde_json::Value) -> Result<Value> {
        let param: TickerScreenParam = serde_json::from_value(value.clone())
            .map_err(|e| anyhow::anyhow!("Failed to deserialize params: {:?} — {:?}", value, e))?;
        info!("Screening Tools param: {:#?}", param);

        let symbols = self.stocks_service.screen_tickers(param).await?;

        info!("Screened stocks: {:?}", symbols);
        Ok(json!({ "sreened_tickers": symbols }))
    }
}
