use crate::{mongo::service::MongoStorageService, service::TickerStorageService};
use anyhow::Result;
use async_trait::async_trait;
use fin_domain::{
    dto::{ticker_group::TickerGroup, ticker_peer::TickerPeer},
    tickers::{Ticker, TickerFilter},
    utils::data_utils::{assets_cap_label_range, assets_cap_range},
};
use rust_decimal::Decimal;
use rustic_storage::core::{repository::Repository, search::SearchCriteria};
use serde_json::json;
use tracing::debug;

#[async_trait]
impl TickerStorageService for MongoStorageService {
    async fn get_ticker_by_symbol(&self, symbol: &str) -> Result<Ticker> {
        match self.manager.tickers().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.find_by_id(symbol.to_string()).await
            }
            Err(e) => Err(anyhow::anyhow!("Error getting Ticker: {}", e)),
        }
    }

    async fn get_tickers(&self) -> Result<Vec<Ticker>> {
        let criteria = SearchCriteria::new().sort_asc("symbol");
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_tickers_by_symbols(&self, symbols: Vec<String>) -> Result<Vec<Ticker>> {
        let criteria = SearchCriteria::new()
            .in_values("symbol", symbols)
            .sort_asc("symbol");
        debug!("get_tickers_by_symbols: {:#?}", criteria);
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_ticker_peers_by_symbols(
        &self,
        symbols: Vec<String>,
        limit: usize,
    ) -> Result<Vec<TickerPeer>> {
        debug!("symbols: {:?}", symbols);
        match self.manager.tickers().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;

                let pipeline = vec![
                    json!({ "$match": { "symbol": { "$in": &symbols } } }),
                    json!({ "$lookup": {
                        "from": "ticker",
                        "let": { "sec": "$sector", "ind": "$industry", "sym": "$symbol" },
                        "pipeline": [
                            { "$match": { "$expr": { "$and": [
                                { "$eq": ["$sector", "$$sec"] },
                                { "$ne": ["$symbol", "$$sym"] }
                            ]}}},
                            { "$addFields": {
                                "score": { "$cond": [
                                    { "$eq": ["$industry", "$$ind"] },
                                    2,
                                    1
                                ]}
                            }},
                            { "$sort": { "score": -1 } },
                            { "$limit": limit }
                        ],
                        "as": "peer_docs"
                    }}),
                    json!({ "$project": {
                        "symbol": 1,
                        "peers": {
                            "$map": {
                                "input": "$peer_docs",
                                "as": "peer",
                                "in": {
                                    "symbol": "$$peer.symbol",
                                    "score": "$$peer.score"
                                }
                            }
                        },
                        "_id": 0
                    }}),
                ];

                let results = repo.aggregate(pipeline).await?;
                let peers: Vec<TickerPeer> = results
                    .iter()
                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                    .collect();
                Ok(peers)
            }
            Err(e) => Err(anyhow::anyhow!("Error getting Ticker: {}", e)),
        }
    }

    async fn get_ticker_groups(&self) -> Result<Vec<TickerGroup>> {
        match self.manager.tickers().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;

                let pipeline = vec![
                    json!({ "$group": { "_id": { "sector": "$sector", "industry": "$industry" } } }),
                    json!({ "$match": { "_id.sector": { "$ne": null }, "_id.industry": { "$ne": null } } }),
                ];

                let results = repo.aggregate(pipeline).await?;

                let groups: Vec<TickerGroup> = results
                    .iter()
                    .filter_map(|v| {
                        serde_json::from_value(json!({
                            "sector": v["_id"]["sector"],
                            "industry": v["_id"]["industry"]
                        }))
                        .ok()
                    })
                    .collect();

                Ok(groups)
            }
            Err(e) => Err(anyhow::anyhow!("Error getting Ticker: {}", e)),
        }
    }

    async fn get_ticker_by_sector(&self, sector: &str) -> Result<Vec<Ticker>> {
        let criteria = SearchCriteria::new()
            .eq("sector", sector.to_uppercase())
            .sort_desc("total_assets");
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_tickers_by_marketcap(&self) -> Result<Vec<Ticker>> {
        let criteria = SearchCriteria::new().sort_desc("total_assets");
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_tickers_by_top_gainers(&self, asset_type: Option<String>) -> Result<Vec<Ticker>> {
        let mut criteria = SearchCriteria::new().sort_desc("pr_diff_perc").limit(20);
        if let Some(asset_type) = asset_type {
            criteria = criteria.eq("asset_type", asset_type.to_uppercase());
        }
        self.get_ticker_by_criteria(&criteria).await
    }
    async fn get_tickers_by_top_gainers_ytd(
        &self,
        asset_type: Option<String>,
    ) -> Result<Vec<Ticker>> {
        let mut criteria = SearchCriteria::new()
            .sort_desc("performance_search.Ytd.perc")
            .limit(20);
        if let Some(asset_type) = asset_type {
            criteria = criteria.eq("asset_type", asset_type.to_uppercase());
        }
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_tickers_by_top_losers(&self, asset_type: Option<String>) -> Result<Vec<Ticker>> {
        let mut criteria = SearchCriteria::new().sort_asc("pr_diff_perc").limit(20);
        if let Some(asset_type) = asset_type {
            criteria = criteria.eq("asset_type", asset_type.to_uppercase());
        }
        self.get_ticker_by_criteria(&criteria).await
    }
    async fn get_tickers_by_top_losers_ytd(
        &self,
        asset_type: Option<String>,
    ) -> Result<Vec<Ticker>> {
        let mut criteria = SearchCriteria::new()
            .sort_asc("performance_search.Ytd.perc")
            .limit(20);
        if let Some(asset_type) = asset_type {
            criteria = criteria.eq("asset_type", asset_type.to_uppercase());
        }
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_ticker_peers_by_industry(&self, symbol: &str) -> Result<Vec<Ticker>> {
        let ticker = self.get_ticker_by_symbol(symbol).await?;
        let industry = ticker.industry.unwrap_or_default();
        if industry.is_empty() {
            return Ok(Vec::new());
        }
        // Get reference ticker market cap bucket
        let (min_cap, max_cap) = assets_cap_range(ticker.total_assets);

        let criteria = SearchCriteria::new()
            .eq("industry", industry)
            .gte("total_assets", min_cap)
            .lte("total_assets", max_cap)
            .sort_desc("total_assets");
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_ticker_peers_by_sector(&self, symbol: &str) -> Result<Vec<Ticker>> {
        let ticker = self.get_ticker_by_symbol(symbol).await?;
        let sector = ticker.sector.unwrap_or_default();
        if sector.is_empty() {
            return Ok(Vec::new());
        }
        // Get reference ticker market cap bucket
        let (min_cap, max_cap) = assets_cap_range(ticker.total_assets);

        let criteria = SearchCriteria::new()
            .eq("sector", sector)
            .gte("total_assets", min_cap)
            .lte("total_assets", max_cap)
            .sort_desc("total_assets");

        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_ticker_overview_embeddings(&self) -> Result<Vec<(Ticker, Vec<f32>)>> {
        let tickers = self.get_tickers().await?;
        let candidates = tickers
            .into_iter()
            .filter_map(|t| t.overview_embedding.clone().map(|embedding| (t, embedding)))
            .collect();

        Ok(candidates)
    }

    async fn get_ticker_industry_embeddings(&self) -> Result<Vec<(Ticker, Vec<f32>)>> {
        let tickers = self.get_tickers().await?;
        let candidates = tickers
            .into_iter()
            .filter_map(|t| t.industry_embedding.clone().map(|embedding| (t, embedding)))
            .collect();

        Ok(candidates)
    }

    async fn search_tickers(&self, filter: TickerFilter) -> Result<Vec<Ticker>> {
        let mut criteria = SearchCriteria::new();
        if let Some(industry) = filter.industry {
            criteria = criteria.contains("industry", industry);
        }

        let new_asset_type = filter
            .asset_type
            .unwrap_or_else(|| "stock".to_string())
            .to_uppercase();

        criteria = criteria.eq("asset_type", new_asset_type);
        if let Some(range) = filter.assets_cap_range {
            let (min_cap, max_cap) = assets_cap_label_range(Some(range));

            criteria = criteria.gte("total_assets", min_cap);
            criteria = criteria.lte("total_assets", max_cap);
        }
        if let Some(signals) = filter.signals {
            criteria = criteria.gte("signals", signals);
        }

        if let Some(cyield) = filter.r#yield
            && cyield > 0.0
        {
            let dec_yield: Decimal = Decimal::from_f32_retain(cyield).unwrap();
            let dec_yield = dec_yield / Decimal::from(100);

            criteria = criteria.gte("yield", dec_yield);
        }

        if let Some(limit) = filter.limit {
            criteria = criteria.limit(limit);
        }

        criteria = criteria.sort_desc("total_assets");

        debug!("search_tickers criteria: {:#?}", criteria);

        self.get_ticker_by_criteria(&criteria).await
    }

    // save ticker or return error
    async fn save_ticker(&self, ticker: Ticker) -> Result<()> {
        match self.manager.tickers().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.update(ticker).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!(format!(
                    "Error saving Ticker for '{}' error: {}",
                    ticker.symbol, e
                )));
            }
        }
    }

    async fn save_tickers(&self, tickers: Vec<Ticker>) -> Result<()> {
        match self.manager.tickers().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.bulk_update(tickers).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!(format!("Error saving Ticker: {}", e)));
            }
        }
    }
}
