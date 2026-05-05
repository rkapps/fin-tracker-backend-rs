use agentic_boot::services::AgentService;
use agentic_core::client::{
    llm::CompletionStreamResponse, message::Message, response::CompletionResponse,
};
use anyhow::Result;
use std::sync::Arc;
use tracing::info;

pub struct AnalyseService {
    agent_service: Arc<AgentService>,
}

impl AnalyseService {
    pub fn new(agent_service: Arc<AgentService>) -> Self {
        Self { agent_service }
    }
    pub async fn analyse_tickers(
        &self,
        llm: &str,
        model: &str,
        prompt: &str,
        response_id: Option<String>,
    ) -> Result<CompletionResponse> {
        let mut messages = vec![];
        let message = Message::User {
            content: prompt.to_string(),
            response_id,
        };
        messages.push(message);

        let agent = self
            .agent_service
            .build_agent_for_id("finance-agent", llm, model)
            .await?;

        let response = agent.complete_with_tools(&messages).await?;
        Ok(response)
    }

    pub async fn analyse_tickers_streaming(
        &self,
        llm: &str,
        model: &str,
        prompt: &str,
        response_id: Option<String>,
    ) -> Result<CompletionStreamResponse> {
        let mut messages = vec![];
        let message = Message::User {
            content: prompt.to_string(),
            response_id,
        };
        messages.push(message);
        info!(
            "analyse ticker prompt: llm: {} model: {} prompt {:?}",
            llm, model, prompt
        );

        let agent = self
            .agent_service
            .build_agent_for_id("finance-agent", llm, model)
            .await?;
        let stream = agent.complete_with_tools_streaming(&messages).await?;
        Ok(Box::pin(stream))
    }
}
