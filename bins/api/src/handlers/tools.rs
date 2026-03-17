use std::{convert::Infallible, sync::Arc};

use agentic_core::capabilities::completion::response::CompletionResponseContent;
use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Sse, sse::Event},
};
use fin_services::tools::ToolsService;
use futures::StreamExt;
use reqwest::StatusCode;
use tracing::{debug, info};

use crate::handlers::stocks::{TickerAnalyseParam, TickerAnalyseResponse};

pub async fn analyse_tickers_handler(
    State(tools_service): State<Arc<ToolsService>>,
    Json(param): Json<TickerAnalyseParam>,
) -> Result<Json<TickerAnalyseResponse>, (StatusCode, String)> {
    debug!("analyse params: {:?}", param);
    let response_id = param.prev_response_id;

    let response = tools_service
        .analyse_tickers(&param.prompt, response_id)
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

pub async fn analyse_tickers_streaming_handler(
    State(tools_service): State<Arc<ToolsService>>,
    Json(param): Json<TickerAnalyseParam>,
) -> impl IntoResponse {
    debug!("started analyse_tickers_streaming_handler");

    let response_id = param.prev_response_id;

    let stream = match tools_service
        .analyse_tickers_streaming(&param.prompt, response_id)
        .await
    {
        Ok(stream) => stream,
        Err(_) => {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let event_stream = stream.map(move |chunk_result| {
        match chunk_result {
            Ok(chunk) => {
                // Convert your ChatResponseChunk to SSE Event
                // info!("chunk: {:?}", chunk);

                match serde_json::to_string(&chunk) {
                    Ok(c) => {
                        Ok::<Event, Infallible>(
                            Event::default()
                                .data(c) // Serialize to JSON string
                                .event("message"),
                        )
                    }
                    Err(e) => Ok::<Event, Infallible>(
                        Event::default().data(format!("{}", e)).event("error"),
                    ),
                }
            }
            Err(e) => {
                // Send error as SSE event
                debug!("error: {:?}", e);
                Ok::<Event, Infallible>(Event::default().data(format!("{}", e)).event("error"))
            }
        }
    });

    Sse::new(event_stream).into_response()
}
