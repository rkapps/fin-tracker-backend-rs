use agentic_core::{
    agent::{builder::Preset, completion::Agent, provider::Provider, service::{AgentService, LlmProvider}},
    client::{
        embeddings::{Embedding, EmbeddingClient},
        llm::CompletionStreamResponse,
        message::Message,
        response::CompletionResponse,
        tools::Tool,
    },
};
use anyhow::Result;
use fin_core::tools::{
    TickerIndicatorTool, TickerPeersTool, TickerPriceHistoryTool, TickerScreeningTool,
    TickerSentimentTool, TickerSnapshotTool, TickerTaxonomyTool,
};
use fin_storage::service::StorageService;
use std::sync::Arc;
use tracing::info;

pub struct AnalyseService {
    storage_service: Arc<dyn StorageService>,
    embedding_client: Arc<dyn EmbeddingClient>,
    agent_service: Arc<AgentService>,
}

impl AnalyseService {
    pub fn new(
        storage_service: Arc<dyn StorageService>,
        embedding_client: Arc<dyn EmbeddingClient>,
        agent_service: Arc<AgentService>,
    ) -> Self {
        Self {
            storage_service,
            embedding_client,
            agent_service,
        }
    }

     /// Returns configured LLM providers — UI uses this for dropdown
    pub fn get_llm_providers(&self) -> Vec<LlmProvider> {
        self.agent_service.get_llm_providers()
    }

    pub async fn analyse_tickers(
        &self,
        llm: &str,
        prompt: &str,
        response_id: Option<String>,
    ) -> Result<CompletionResponse> {
        let mut messages = vec![];
        let message = Message::User {
            content: prompt.to_string(),
            response_id,
        };
        messages.push(message);

        let agent = self.build_agent(llm, prompt).await?;
        let system_prompt = self.build_system_prompt();
        let response = agent.complete_with_tools(&system_prompt, &messages).await?;
        Ok(response)
    }

    pub async fn analyse_tickers_streaming(
        &self,
        llm: &str,
        prompt: &str,
        response_id: Option<String>,
    ) -> Result<CompletionStreamResponse> {
        let mut messages = vec![];
        let message = Message::User {
            content: prompt.to_string(),
            response_id,
        };
        messages.push(message);
        info!("analyse ticker prompt: llm: {} {:?}", llm, messages);

        let agent = self.build_agent(llm, prompt).await?;
        let system_prompt = self.build_system_prompt();
        let stream = agent
            .complete_with_tools_streaming(&system_prompt, &messages)
            .await?;
        Ok(Box::pin(stream))
    }

    async fn build_agent(&self, llm: &str, prompt: &str) -> Result<Agent> {
        // get the input embeddings for the prompt
        let query_embedding = self
            .embedding_client
            .embed_text(prompt)
            .await
            .map_err(|e| anyhow::anyhow!("Error embedding input prompt {}: {}", prompt, e))?;

        // resolve_provider has everything it needs — no keys passed in
        let provider = self.agent_service.resolve_provider(llm)?;

        // For a non local agent, use thorough
        let preset = match &provider {
            Provider::Local { .. } => Preset::Local,
            _ => Preset::Thorough,
        };

        let agent = self
            .agent_service
            .builder()
            .with_tools(self.build_finance_tools(query_embedding))
            .with_preset(preset)
            .with_provider(provider)?
            .build()?;

        Ok(agent)
    }

    fn build_finance_tools(&self, query_embedding: Embedding) -> Vec<Box<dyn Tool>> {
        vec![
            Box::new(TickerScreeningTool::new(
                self.storage_service.clone(),
                self.embedding_client.clone(),
            )),
            Box::new(TickerTaxonomyTool::new(self.storage_service.clone())),
            Box::new(TickerSentimentTool::new(
                query_embedding.clone(),
                self.storage_service.clone(),
            )),
            Box::new(TickerSnapshotTool::new(self.storage_service.clone())),
            Box::new(TickerPriceHistoryTool::new(self.storage_service.clone())),
            Box::new(TickerIndicatorTool::new(self.storage_service.clone())),
            Box::new(TickerPeersTool::new(self.storage_service.clone())),
        ]
    }

    fn build_system_prompt(&self) -> Option<String> {
        /*
                let system_prompt = Some("You are financial expert and advisor in analysing stocks and market trends. You will help guide my decision making in the stock market. Use the provide tool if necessary to get information on the stocks".to_string());
        */
        Some(
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
        )
    }
}
