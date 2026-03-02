use std::sync::Arc;
use axum::extract::FromRef;
use fin_services::{stocks::StocksService, tools::ToolsService};


#[derive(Clone)]
pub struct AppState {
    pub stocks_service: Arc<StocksService>,
    pub tools_service: Arc<ToolsService>,
}



#[derive(Clone)]
pub struct OpenAIApiKey(pub String);

#[derive(Clone)]
pub struct GeminiApiKey(pub String);

#[derive(Clone)]
pub struct AnthropicApiKey(pub String);



impl FromRef<AppState> for Arc<ToolsService> {
    fn from_ref(state: &AppState) -> Self {
        state.tools_service.clone()
    }
}

impl FromRef<AppState> for Arc<StocksService> {
    fn from_ref(state: &AppState) -> Self {
        state.stocks_service.clone()
    }
}
