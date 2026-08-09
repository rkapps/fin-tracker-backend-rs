use anyhow::Result;
use rustic_boot::BootState;
use std::{convert::Infallible, sync::Arc, vec};

use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Sse, sse::Event},
};
use futures::StreamExt;
use reqwest::StatusCode;
use rustic_agent::{
    CompletionResponse, CompletionResponseContent, CompletionStreamResponse,
    agents::domain::{AgentInput, LlmConfig},
};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, error, info};

#[derive(Deserialize, Debug)]
pub struct TickerAnalyseParam {
    pub llm: String,
    pub model: String,
    pub prompt: String,
    pub prev_response_id: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct TickerAnalyseResponse {
    pub content: String,
    pub response_id: String,
}

pub async fn analyse_tickers_handler(
    State(boot): State<Arc<BootState>>,
    Json(param): Json<TickerAnalyseParam>,
) -> Result<Json<TickerAnalyseResponse>, (StatusCode, String)> {
    debug!("analyse params: {:?}", param);

    let response = analyse_tickers(boot, &param.llm, &param.model, &param.prompt)
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
}

pub async fn analyse_tickers_streaming_handler(
    State(boot): State<Arc<BootState>>,
    Json(param): Json<TickerAnalyseParam>,
) -> impl IntoResponse {
    debug!("started analyse_tickers_streaming_handler");


    let stream =
        match analyse_tickers_streaming(boot, &param.llm, &param.model, &param.prompt)
            .await
        {
            Ok(stream) => stream,
            Err(e) => {
                error!("analyse_tickers_streaming error: {}", e.to_string());
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        };

    let final_content = Arc::new(Mutex::new(String::new()));
    let final_thought = Arc::new(Mutex::new(String::new()));

    let event_stream = stream.then(move |chunk_result| {
        // ✅ Clone handles into the async block
        let final_content = final_content.clone();
        let final_thought = final_thought.clone();

        async move {
            match chunk_result {
                Ok(chunk) => {
                    // ✅ Always accumulate content once (was being doubled before)
                    {
                        let mut fc = final_content.lock().await;
                        fc.push_str(&chunk.content);

                        let mut ft = final_thought.lock().await;
                        ft.push_str(&chunk.thought);
                    }

                    // ✅ Save only on the final chunk
                    if chunk.is_final {
                        let fc = final_content.lock().await;
                        let ft = final_thought.lock().await;
                        info!("final_thought: {:?}", *ft);
                        info!("final_content: {:?}", *fc);
                    }

                    match serde_json::to_string(&chunk) {
                        Ok(c) => Ok::<Event, Infallible>(Event::default().data(c).event("message")),
                        Err(e) => Ok::<Event, Infallible>(
                            Event::default().data(format!("{}", e)).event("error"),
                        ),
                    }
                }
                Err(e) => {
                    debug!("error: {:?}", e);
                    Ok::<Event, Infallible>(Event::default().data(format!("{}", e)).event("error"))
                }
            }
        }
    });

    Sse::new(event_stream).into_response()
}

pub async fn analyse_tickers(
    boot: Arc<BootState>,
    llm: &str,
    model: &str,
    prompt: &str,
) -> Result<CompletionResponse> {

    info!(
        "analyse ticker prompt: llm: {} model: {} prompt {:?}",
        llm, model, prompt
    );

    let llm_config = LlmConfig {
        llm: Some(llm.to_string()),
        model: Some(model.to_string()),
        ..Default::default()
    };
    let agent_input = AgentInput::new(
        "finance-analyser".to_string(),
        llm_config,
        None,
        false,
    );

    let runner = boot
        .agent_service
        .build_runnable(&agent_input)
        .await?;
    let cresponse = runner.execute(vec![], &prompt, false).await?;
    // let response = agent.execute(&input).await?;
    Ok(cresponse)
}

pub async fn analyse_tickers_streaming(
    boot: Arc<BootState>,
    llm: &str,
    model: &str,
    prompt: &str,
) -> Result<CompletionStreamResponse> {
    
    info!(
        "analyse ticker prompt: llm: {} model: {} prompt {:?}",
        llm, model, prompt
    );

    let llm_config = LlmConfig {
        llm: Some(llm.to_string()),
        model: Some(model.to_string()),
        ..Default::default()
    };
    let agent_input = AgentInput::new(
        "finance-analyser".to_string(),
        llm_config,
        None,
        false,
    );

    let runner = boot
        .agent_service
        .build_runnable(&agent_input)
        .await?;

    let stream = runner.execute_streaming(vec![], &prompt, false).await?;
    Ok(Box::pin(stream))
}
