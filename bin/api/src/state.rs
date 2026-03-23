use std::sync::Arc;
use axum::extract::FromRef;
use fin_services::{ml::service::MlService, stocks::StocksService, ticker_service::TickerService, tools::ToolsService};


#[derive(Clone)]
pub struct AppState {
    pub stocks_service: Arc<StocksService>,
    pub ticker_service: Arc<TickerService>,
    pub tools_service: Arc<ToolsService>,
    pub ml_service: Arc<MlService>,
}


#[derive(Clone)]
pub struct OpenAIApiKey(pub String);

#[derive(Clone)]
pub struct GeminiApiKey(pub String);

#[derive(Clone)]
pub struct AnthropicApiKey(pub String);


impl FromRef<AppState> for Arc<TickerService> {
    fn from_ref(state: &AppState) -> Self {
        state.ticker_service.clone()
    }
}

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

impl FromRef<AppState> for Arc<MlService> {
    fn from_ref(state: &AppState) -> Self {
        state.ml_service.clone()
    }
}
