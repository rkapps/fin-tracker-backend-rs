use std::sync::Arc;
use agentic_core::agent::service::AgentService;
use axum::extract::FromRef;
use fin_services::stocks::StocksService;


#[derive(Clone)]
pub struct AppState {
    pub stocks_service: Arc<StocksService>,
    pub agent_service: Arc<AgentService>,
    pub openai_api_key: OpenAIApiKey,
    pub gemini_api_key: GeminiApiKey,
    pub anthropic_api_key: AnthropicApiKey
}



#[derive(Clone)]
pub struct OpenAIApiKey(pub String);

#[derive(Clone)]
pub struct GeminiApiKey(pub String);

#[derive(Clone)]
pub struct AnthropicApiKey(pub String);



impl FromRef<AppState> for Arc<StocksService> {
    fn from_ref(state: &AppState) -> Self {
        state.stocks_service.clone()
    }
}

impl FromRef<AppState> for Arc<AgentService> {
    fn from_ref(state: &AppState) -> Self {
        state.agent_service.clone()
    }
}

impl FromRef<AppState> for OpenAIApiKey {
    fn from_ref(state: &AppState) -> Self {
        state.openai_api_key.clone()
    }
}

impl FromRef<AppState> for GeminiApiKey {
    fn from_ref(state: &AppState) -> Self {
        state.gemini_api_key.clone()
    }
}

impl FromRef<AppState> for AnthropicApiKey {
    fn from_ref(state: &AppState) -> Self {
        state.anthropic_api_key.clone()
    }
}