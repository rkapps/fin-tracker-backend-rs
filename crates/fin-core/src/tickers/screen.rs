// use std::sync::Arc;

// use agentic_core::client::embeddings::EmbeddingClient;
// use anyhow::Result;
// use fin_domain::tickers::{Ticker, TickerFilter};
// use fin_domain::utils::data_utils::get_overview_embeddings;
// use fin_storage::service::StorageService;
// use storage_core::vector::search;
// use tracing::debug;

// pub async fn screen_tickers(
//     storage_service: Arc<dyn StorageService>,
//     embedding_client: Arc<dyn EmbeddingClient>,
//     filter: TickerFilter,
// ) -> Result<Vec<String>> {
//     let tickers = storage_service.search_tickers(filter.clone()).await?;
//     debug!("Screened stocks from initial search: {}", tickers.len());

//     let overview_candidates: Vec<(Ticker, Vec<f32>)> = get_overview_embeddings(&tickers);
//     debug!("Overview candidates: {}", overview_candidates.len());
//     let limit = filter.limit.unwrap_or(10);

//     let symbols: Vec<String> = if let Some(query) = filter.query {
//         let vectors = embedding_client.embed_text(&query).await?.into_vec();

//         let candidates: Vec<(String, Vec<f32>)> = overview_candidates
//             .iter()
//             .map(|(t, e)| (t.symbol.clone(), e.clone()))
//             .collect();

//         search::search(&vectors, &candidates, limit)
//             .into_iter()
//             .map(|(s, _)| s)
//             .collect()
//     } else {
//         tickers.into_iter().take(limit).map(|t| t.symbol).collect()
//     };
//     debug!("Symbols: {:?}", symbols);

//     Ok(symbols)
// }
