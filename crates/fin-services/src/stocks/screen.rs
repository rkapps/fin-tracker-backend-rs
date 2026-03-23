use crate::stocks::StocksService;
use anyhow::Result;
use fin_domain::{
    dto::screen_param::TickerScreenParam, ticker::Ticker,
    utils::data_utils::get_overview_embeddings,
};
use storage_core::vector::search;
use tracing::debug;

impl StocksService {
    pub async fn screen_tickers(
        &self,
        param: TickerScreenParam,
        // industry: Option<String>,
        // market_cap_range: Option<String>, // "mega", "large", "mid", "small"
        // asset_type: Option<String>,
        // signals: Option<Vec<String>>
    ) -> Result<Vec<String>> {
        let tickers = self
            .storage_service
            .search_tickers(
                param.clone(), // params.industry,
                               // params.market_cap_range,
                               // params.asset_type,
                               // params.signals,
            )
            .await?;
        debug!("Screened stocks from initial search: {}", tickers.len());

        let overview_candidates: Vec<(Ticker, Vec<f32>)> = get_overview_embeddings(&tickers);
        debug!("Overview candidates: {}", overview_candidates.len());
        let limit = param.limit.unwrap_or(10);

        let symbols: Vec<String> = if let Some(query) = param.query {
            let vectors = self.embedding_client.embed_text(&query).await?.into_vec();

            let candidates: Vec<(String, Vec<f32>)> = overview_candidates
                .iter()
                .map(|(t, e)| (t.symbol.clone(), e.clone()))
                .collect();

            search::search(&vectors, &candidates, limit)
                .into_iter()
                .map(|(s, _)| s)
                .collect()
        } else {
            tickers.into_iter().take(limit).map(|t| t.symbol).collect()
        };
        debug!("Symbols: {:?}", symbols);

        Ok(symbols)
    }
}
