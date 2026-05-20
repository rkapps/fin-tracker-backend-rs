use axum::extract::FromRef;
use fin_services::{analyse::AnalyseService, ticker::TickersService};
use rustic_boot::BootState;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub boot_state: Arc<BootState>,
    pub ticker_service: Arc<TickersService>,
    pub analyse_service: Arc<AnalyseService>,
}
impl FromRef<AppState> for Arc<BootState> {
    fn from_ref(state: &AppState) -> Arc<BootState> {
        state.boot_state.clone()
    }
}

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
