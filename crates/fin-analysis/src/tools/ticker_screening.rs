use agentic_core::capabilities::client::{embeddings::EmbeddingClient, tool::Tool};
use anyhow::Result;
use async_trait::async_trait;
use fin_domain::{ticker::Ticker, utils::data_utils::get_overview_embeddings};
use fin_storage::service::StorageService;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use storage_core::vector::search;
use tracing::{debug, info};

#[derive(Debug)]
pub struct TickerScreeningTool {
    embedding_client: Arc<dyn EmbeddingClient>,
    storage_service: Arc<dyn StorageService>,
}
impl TickerScreeningTool {
    pub fn new(
        embedding_client: Arc<dyn EmbeddingClient>,
        storage_service: Arc<dyn StorageService>,
    ) -> TickerScreeningTool {
        Self {
            embedding_client,
            storage_service,
        }
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
                - Only include market_cap_range if the user explicitly mentions a market cap size. Never infer it. \
                - When the query implies a specific industry, populate both query and industry fields. \
        \
        Examples: \
            'software infrastructure mid cap stocks' → query: 'software infrastructure', industry: 'Software - Infrastructure', market_cap_range: 'mid' \
            'find oversold cloud security companies' → query: 'cloud security', signals: ['RSI Oversold'] \
            'defensive buy rated stocks' → signals: ['Low Beta', 'Analyst Buy'] \
            'compare spider ETFs' → query: 'SPDR ETFs', asset_type: 'etf' \
        \
        Signal rules: \
            Pass ONE signal per category. Results must match ALL signals provided. \
            Categories: \
                Trend: 'Golden Cross', 'Death Cross', 'Above SMA50', 'Below SMA50'. \
                Momentum: 'MACD Bullish Crossover', 'MACD Bearish Crossover'. \
                RSI: 'RSI Oversold', 'RSI Overbought'. \
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
                            "Mean Reversion Candidate",
                            "Momentum Breakout", "Bullish Trend Exhaustion", "Bearish Trend Exhaustion",
                            "Volatility Expanding", "Volatility Contracting",
                            "Analyst Strong Buy", "Analyst Buy", "Analyst Hold",
                            "Analyst Sell", "Analyst Strong Sell", 
                            "Low Beta", "Market Beta", "High Beta", "Very High Beta"
                        ]
                    },
                    "description": "Filter by active technical or analyst signals."
                },
                "industry": {
                    "type": "string",
                    "description": "Filter by industry, partial match. Example: 'semiconductors', 'medical devices'."
                },
                "market_cap_range": {
                    "type": "string",
                    "enum": ["mega", "large", "mid", "small"],
                    "description": "Filter by market cap. mega: >$1T, large: $100B-$1T, mid: $2B-$100B, small: <$2B. \
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
        #[derive(Debug, Deserialize)]
        struct ScreenParams {
            query: Option<String>, // semantic: "cloud security", "payments infrastructure"
            signals: Option<Vec<String>>, // ["RSI Oversold", "MACD Bullish Crossover"]
            industry: Option<String>, // regex match
            market_cap_range: Option<String>, // "mega", "large", "mid", "small"
            asset_type: Option<String>, // "stock", "etf"
            limit: Option<usize>,
        }

        let params: ScreenParams = serde_json::from_value(value.clone())
            .map_err(|e| anyhow::anyhow!("Failed to deserialize params: {:?} — {:?}", value, e))?;
        info!("Screening Tools param: {:#?}", params);

        let tickers = self
            .storage_service
            .search_tickers(
                params.industry,
                params.market_cap_range,
                params.asset_type,
                params.signals,
            )
            .await?;
        debug!("Screened stocks from initial search: {}", tickers.len());

        let overview_candidates: Vec<(Ticker, Vec<f32>)> = get_overview_embeddings(&tickers);
        let limit = params.limit.unwrap_or(10);

        let symbols: Vec<String> = if let Some(query) = params.query {
            let vectors = self.embedding_client.embed_text(&query).await?.into_vec();

            let candidates: Vec<(String, Vec<f32>)> = overview_candidates
                .iter()
                .map(|(t, e)| (t.symbol.clone(), e.clone()))
                .collect();

            search::search(&vectors, &candidates, limit)
                .into_iter()
                .map(|(s, _)| s)
                .collect()
        } else {
            tickers.into_iter().take(limit).map(|t| t.symbol).collect()
        };

        info!("Screened stocks: {:?}", symbols);
        Ok(json!({ "sreened_tickers": symbols }))
    }
}
