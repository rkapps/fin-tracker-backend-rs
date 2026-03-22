use agentic_core::{
    agent::{completion::Agent, service::AgentService}, client::{embeddings::EmbeddingClient, llm::CompletionStreamResponse, message::Message, response::CompletionResponse}, providers::{anthropic::{MODEL_CLAUDE_OPUS_4_6, MODEL_CLAUDE_SONNET_4_6}, gemini::MODEL_GEMINI_3_FLASH_PREVIEW},
};
use anyhow::Result;
use serde::Deserialize;
use tracing::info;
use std::sync::Arc;

use crate::stocks::StocksService;

mod ticker_indicator;
pub mod ticker_peers;
pub mod ticker_price_history;
pub mod ticker_screening;
pub mod ticker_sentiment;
pub mod ticker_similarity;
pub mod ticker_snapshot;
pub mod ticker_taxonomy;

pub use ticker_indicator::TickerIndicatorTool;
pub use ticker_peers::TickerPeersTool;
pub use ticker_price_history::TickerPriceHistoryTool;
pub use ticker_screening::TickerScreeningTool;
pub use ticker_sentiment::TickerSentimentTool;
pub use ticker_snapshot::TickerSnapshotTool;
pub use ticker_taxonomy::TickerTaxonomyTool;

#[derive(Debug, Deserialize)]
pub struct TickerParam {
    symbol: String,
}

pub struct ToolsService {
    pub stocks_service: Arc<StocksService>,
    embedding_client: Arc<dyn EmbeddingClient>,
    agent_service: Arc<AgentService>,
    #[allow(dead_code)]
    openai_api_key: String,
    #[allow(dead_code)]
    gemini_api_key: String,
    #[allow(dead_code)]
    anthropic_api_key: String,
}

impl ToolsService {
    pub fn new(
        stocks_service: Arc<StocksService>,
        embedding_client: Arc<dyn EmbeddingClient>,
        agent_service: Arc<AgentService>,
        openai_api_key: String,
        gemini_api_key: String,
        anthropic_api_key: String,
    ) -> Self {
        Self {
            stocks_service,
            embedding_client,
            agent_service,
            openai_api_key,
            gemini_api_key,
            anthropic_api_key,
        }
    }

    pub async fn analyse_tickers(
        &self,
        prompt: &str,
        response_id: Option<String>,
    ) -> Result<CompletionResponse> {
        let mut messages = vec![];
        let message = Message::User {
            content: prompt.to_string(),
            response_id: response_id,
        };
        messages.push(message);

        let agent = self.build_agent(prompt).await?;
        let system_prompt = self.build_system_prompt();
        let response = agent.complete_with_tools(&system_prompt, &messages).await?;
        Ok(response)
    }

    pub async fn analyse_tickers_streaming(
        &self,
        prompt: &str,
        response_id: Option<String>,
    ) -> Result<CompletionStreamResponse> {
        let mut messages = vec![];
        let message = Message::User {
            content: prompt.to_string(),
            response_id: response_id,
        };
        messages.push(message);
        info!("analyse ticker prompt: {:?}", messages);

        let agent = self.build_agent(prompt).await?;
        let system_prompt = self.build_system_prompt();
        // let system_prompt = Some("You are a expert at everthing".to_string());
        let stream = agent
            .complete_with_tools_streaming(&system_prompt, &messages)
            .await?;
        Ok(Box::pin(stream))
    }

    async fn build_agent(&self, prompt: &str) -> Result<Agent> {
        // get the input embeddings for the prompt
        let query_embedding = self
            .embedding_client
            .embed_text(prompt)
            .await
            .map_err(|e| anyhow::anyhow!("Error embedding input prompt {}: {}", prompt, e))?;

        let taxonomy_tool = TickerTaxonomyTool::new(self.stocks_service.storage_service.clone());
        let sentiment_tool = TickerSentimentTool::new(
            query_embedding.clone(),
            self.stocks_service.storage_service.clone(),
        );
        let screening_tool = TickerScreeningTool::new(self.stocks_service.clone());
        // let simiarity_tool =
        //     TickerSimilarityTool::new(query_embedding, self.storage_service.clone());
        let snapshot_tool = TickerSnapshotTool::new(self.stocks_service.storage_service.clone());
        let history_tool = TickerPriceHistoryTool::new(self.stocks_service.storage_service.clone());
        let indicator_tool = TickerIndicatorTool::new(self.stocks_service.storage_service.clone());
        let peers_tool = TickerPeersTool::new(self.stocks_service.storage_service.clone());

        let agent = self
            .agent_service
            .builder()
            // .with_openai(&self.openai_api_key, MODEL_GPT_5_4_MINI)?
            .with_gemini(&self.gemini_api_key, MODEL_GEMINI_3_FLASH_PREVIEW)?
            // .with_anthropic(&self.anthropic_api_key, MODEL_CLAUDE_OPUS_4_6)?
            // .with_anthropic(&self.anthropic_api_key, MODEL_CLAUDE_SONNET_4_6)?

            .with_preset_thorough()
            .with_tool(screening_tool)
            .with_tool(taxonomy_tool)
            // .with_tool(simiarity_tool)
            .with_tool(sentiment_tool)
            .with_tool(snapshot_tool)
            .with_tool(history_tool)
            .with_tool(indicator_tool)
            .with_tool(peers_tool)
            // .with_temperature(0.1)
            // .with_max_tokens(3000)
            .build()?;

        Ok(agent)
    }

    fn build_system_prompt(&self) -> Option<String> {
        /*
                let system_prompt = Some("You are financial expert and advisor in analysing stocks and market trends. You will help guide my decision making in the stock market. Use the provide tool if necessary to get information on the stocks".to_string());
        */
        let system_prompt = Some(
            "You are an expert financial analyst and advisor. Your role is to provide clear, \
             data-driven analysis to support investment decision making. \
             When a query involves specific stock tickers, always use the available tools to \
             fetch current data — never rely on your training knowledge for prices, indicators, \
             or sentiment. Market data goes stale quickly. \
             Be direct. The user is making financial decisions and needs clarity, not hedging. \
             No bullet points. No notes. No disclaimers. No closing remarks. \
             \
             FETCHING DATA RULES — follow in exact order: \
                1. Call ticker_taxonomy ONLY if the query mentions a specific sector, industry or company type. \
                    Skip for signal-only queries like 'find bullish stocks'. \
                2. Call ALL ticker_screening tools needed in ONE turn simultaneously. \
                3. After ALL screening calls complete, collect every returned ticker into one list. \
                4. In ONE single turn, call snapshot AND indicator AND sentiment for EVERY ticker simultaneously. \
                    Example: if list is [BSX, MDT, ABT] call snapshot(BSX), snapshot(MDT), snapshot(ABT), \
                    indicator(BSX), indicator(MDT), indicator(ABT), sentiment(BSX), sentiment(MDT), sentiment(ABT) \
                    all in the same turn. \
                5. After all data is fetched, generate the response. \
                    NEVER call snapshot, indicator or sentiment one ticker at a time. \
                    NEVER call snapshot in one turn and indicator in the next turn. \
                    NEVER fetch any data before all screening calls are complete.
             \
             TABLE RULES: \
             - Metrics are always rows. Tickers are always columns. \
             - For a single ticker: two columns — Metric | Value. \
             - For multiple tickers: first column is Metric, then one column per ticker. \
             - Always include the markdown separator row between header and data rows. \
             - Example: \
               | Metric     | AAPL    | NVDA    | \
               |------------|---------|---------|  \
               | Price      | 260.58  | 187.90  | \
               | Market Cap | $3.83T  | $4.57T  | \
             - If a row has no data (all cells are N/A or 0 or 0% empty), omit that row entirely. \
             - Apply bold to strong signals: \
               RSI oversold/overbought, MACD crossover, Deeply Oversold, returns above +15% or below -15%, \
               Extremely Bullish/Bearish sentiment, Analyst Strong Buy/Strong Sell. \
             \
             RESPONSE FORMAT: \
             Default is SUMMARY ONLY. Only show DETAIL if the user explicitly says \
             'detail', 'deep dive', 'full breakdown', or 'more information'. \
             \
             SUMMARY — one compact table with these rows only: \
             Sector, Industry, Price, Market Cap, P/E, Beta, MACD, RSI, Bands\
             YTD Return, Analyst Price Target, Analyst Consensus, Sentiment, MLP Signal\
             \
             P/E row: show as 'TTM / Forward' in a single cell and interpret the relationship. \
             Example: '33.45 / 30.21 — multiple compressing, earnings growth expected'. \
             If TTM P/E is 0 or negative: 'N/A / 30.21 — currently unprofitable, expected to turn profitable'. \
             \
             Beta row: interpret the value, do not show the raw number. \
             Below 0.8: 'Low volatility, defensive'. \
             0.8 to 1.2: 'Market-like volatility'. \
             1.2 to 1.5: 'High volatility'. \
             Above 1.5: 'Very High volatility, aggressive'. \
             \
             MACD row: interpretation only. Example: 'Bullish momentum' or 'Bearish momentum'. \
             RSI row: interpretation only. Example: 'Near oversold' or 'Neutral'. \
             Bands row: interpret where price sits relative to Bollinger Bands. \
             Use exactly one of: \
             'Near upper band — overbought pressure', \
             'Near middle band — neutral, consolidating', \
             'Near lower band — oversold pressure', \
             'Above upper band — strongly overbought', \
             'Below lower band — strongly oversold'. \
             \
            MLP Signal: list each MLP signal on a new line within the cell.\
                Only include signals where precision ≥ 55%.\
                If none available skip the ML Signal rows.\
            Example: 'MLP20 Bullish (3.2%) ✅ | MLP60 Bullish (7.3%) ✅'\
                    ML Confluence: the cross-period summary signal if present, e.g.\
            'ML Strong Bull — All Periods Confirmed' or '—' if none.\
            Sentiment must be exactly one of: \
             Extremely Bullish, Bullish, Neutral, Bearish, Extremely Bearish. \
             \
             DETAIL — full table with four row groups as bold headers within a single unified table: \
             FUNDAMENTALS: Price, Market Cap, EPS, P/E, PEG, P/B, P/S, 52W High, 52W Low. Raw values only. \
             TECHNICALS: MACD, RSI, Bollinger Bands, Beta, Trend. Interpretations only, no raw values. \
             PRICE HISTORY: Period, Return, High, Low, Trend. Trend is brief e.g. 'Declining, narrow range'. \
             SENTIMENT: Overall, Key Theme. One sentence per cell. \
             Never split DETAIL into multiple separate tables. \
             \
             After the table, always include a SYNOPSIS on a new line. \
             Start it with 'Synopsis:' on its own line. \
             The SYNOPSIS must be exactly 2-3 sentences. \
             Never include the synopsis inside the table. \
             Bold the key recommendation in the SYNOPSIS. \
             End your response after the SYNOPSIS. Nothing else."
            .to_string()
        );

        system_prompt
    }
}
