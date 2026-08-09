use axum::extract::FromRef;
use rustic_boot::BootState;
use rustic_finance::service::FinanceService;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub boot_state: Arc<BootState>,
    pub finance_service: Arc<FinanceService>,
}
impl FromRef<AppState> for Arc<BootState> {
    fn from_ref(state: &AppState) -> Arc<BootState> {
        state.boot_state.clone()
    }
}

impl FromRef<AppState> for Arc<FinanceService> {
    fn from_ref(state: &AppState) -> Self {
        state.finance_service.clone()
    }
}
