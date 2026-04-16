use axum::extract::FromRef;
use fin_services::{analyse::AnalyseService, ticker::TickersService};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub ticker_service: Arc<TickersService>,
    pub analyse_service: Arc<AnalyseService>,
}

#[derive(Clone)]
pub struct OpenAIApiKey(pub String);

#[derive(Clone)]
pub struct GeminiApiKey(pub String);

#[derive(Clone)]
pub struct AnthropicApiKey(pub String);

impl FromRef<AppState> for Arc<TickersService> {
    fn from_ref(state: &AppState) -> Self {
        state.ticker_service.clone()
    }
}

impl FromRef<AppState> for Arc<AnalyseService> {
    fn from_ref(state: &AppState) -> Self {
        state.analyse_service.clone()
    }
}
