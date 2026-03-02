use std::sync::Arc;

use agentic_core::capabilities::completion::response::CompletionResponseContent;
use axum::{Json, extract::State};
use fin_services::tools::ToolsService;
use reqwest::StatusCode;
use tracing::debug;

use crate::handlers::stocks::{TickerAnalyseParam, TickerAnalyseResponse};


pub async fn analyse_tickers_handler(
    State(tools_service): State<Arc<ToolsService>>,
    Json(param): Json<TickerAnalyseParam>,
) -> Result<Json<TickerAnalyseResponse>, (StatusCode, String)> {
    debug!("analyse params: {:?}", param);
    let response_id = param.prev_response_id;

    let response = tools_service
        .analyse_tickers(
            &param.prompt,
            response_id,
        )
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Analyse ticker error: {}", e),
            )
        })?;

    let aresponse = response
        .contents
        .iter()
        .find_map(|c| {
            if let CompletionResponseContent::Text(val) = c {
                Some(val.clone())
            } else {
                None
            }
        })
        .unwrap_or_default();

    let aresponse = TickerAnalyseResponse {
        content: aresponse,
        response_id: response.response_id,
    };
    Ok(Json(aresponse))
    // Ok(Json(response))
}
