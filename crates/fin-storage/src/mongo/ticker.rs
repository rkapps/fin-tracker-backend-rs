use std::collections::HashMap;

use crate::{mongo::service::MongoStorageService, service::TickerStorageService};
use anyhow::Result;
use async_trait::async_trait;
use fin_domain::{
    tickers::{Ticker, TickerFilter},
    utils::data_utils::{assets_cap_label_range, assets_cap_range},
};
use rust_decimal::Decimal;
use rustic_storage::core::{
    repository::Repository,
    search::{SearchCriteria, SearchOp, SearchValue},
};
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
        // criteria.add_sort("symbol", true);
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_tickers_by_symbols(&self, symbols: Vec<String>) -> Result<Vec<Ticker>> {
        let criteria = SearchCriteria::new()
            .in_values("symbol", symbols)
            .sort_asc("symbol");
        // criteria.add_condition("symbol", SearchOp::In, SearchValue::Array(symbols));
        // criteria.add_sort("symbol", true);
        debug!("get_tickers_by_symbols: {:#?}", criteria);
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_ticker_groups(&self) -> Result<HashMap<String, Vec<String>>> {
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();
        let tickers = self.get_tickers().await?;
        for ticker in tickers {
            if let (Some(sector), Some(industry)) = (ticker.sector, ticker.industry) {
                let industries = groups.entry(sector).or_default();
                if !industries.contains(&industry) {
                    industries.push(industry);
                }
            }
        }
        Ok(groups)
    }

    async fn get_ticker_by_sector(&self, sector: &str) -> Result<Vec<Ticker>> {
        let criteria = SearchCriteria::new()
            .eq("sector", sector.to_uppercase())
            .sort_desc("total_assets");
        // criteria.add_condition(
        //     "sector",
        //     SearchOp::Eq,
        //     SearchValue::String(sector.to_string()),
        // );
        // criteria.add_sort("total_assets", false);
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_tickers_by_marketcap(&self) -> Result<Vec<Ticker>> {
        let criteria = SearchCriteria::new().sort_desc("total_assets");
        // criteria.add_sort("total_assets", false);
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_tickers_by_top_gainers(&self, asset_type: Option<String>) -> Result<Vec<Ticker>> {
        let mut criteria = SearchCriteria::new().sort_desc("pr_diff_perc").limit(20);
        if let Some(asset_type) = asset_type {
            criteria = criteria.eq("asset_type", asset_type.to_uppercase());
            // criteria.add_condition(
            //     "asset_type",
            //     SearchOp::Eq,
            //     SearchValue::String(asset_type.to_uppercase()),
            // );
        }
        // criteria.add_sort("pr_diff_perc", false);
        // criteria.add_limit(20);
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
        // let mut criteria = SearchCriteria::new();
        // if let Some(asset_type) = asset_type {
        //     criteria.add_condition(
        //         "asset_type",
        //         SearchOp::Eq,
        //         SearchValue::String(asset_type.to_uppercase()),
        //     );
        // }
        // criteria.add_sort("performance_search.Ytd.perc", false);
        // criteria.add_limit(20);
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_tickers_by_top_losers(&self, asset_type: Option<String>) -> Result<Vec<Ticker>> {
        let mut criteria = SearchCriteria::new().sort_asc("pr_diff_perc").limit(20);
        if let Some(asset_type) = asset_type {
            criteria = criteria.eq("asset_type", asset_type.to_uppercase());
        }

        // let mut criteria = SearchCriteria::new();
        // if let Some(asset_type) = asset_type {
        //     criteria.add_condition(
        //         "asset_type",
        //         SearchOp::Eq,
        //         SearchValue::String(asset_type.to_uppercase()),
        //     );
        // }
        // criteria.add_sort("pr_diff_perc", true);
        // criteria.add_limit(20);
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
        // let mut criteria = SearchCriteria::new();
        // if let Some(asset_type) = asset_type {
        //     criteria.add_condition(
        //         "asset_type",
        //         SearchOp::Eq,
        //         SearchValue::String(asset_type.to_uppercase()),
        //     );
        // }
        // criteria.add_sort("performance_search.Ytd.perc", true);
        // criteria.add_limit(20);
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
        // criteria.add_condition("industry", SearchOp::Eq, SearchValue::String(industry));
        // criteria.add_condition("total_assets", SearchOp::Gte, SearchValue::Int(min_cap));
        // criteria.add_condition("total_assets", SearchOp::Lte, SearchValue::Int(max_cap));
        // criteria.add_sort("total_assets", false);
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

        // let mut criteria = SearchCriteria::new();
        // criteria.add_condition("sector", SearchOp::Eq, SearchValue::String(sector));
        // criteria.add_condition("total_assets", SearchOp::Gte, SearchValue::Int(min_cap));
        // criteria.add_condition("total_assets", SearchOp::Lte, SearchValue::Int(max_cap));
        // criteria.add_sort("total_assets", false);

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
            // criteria.add_condition(
            //     "industry",
            //     SearchOp::Contains,
            //     SearchValue::String(industry),
            // );
        }

        let new_asset_type = filter
            .asset_type
            .unwrap_or_else(|| "stock".to_string())
            .to_uppercase();

        criteria = criteria.eq("asset_type", new_asset_type);
        // criteria.add_condition(
        //     "asset_type",
        //     SearchOp::Eq,
        //     SearchValue::String(new_asset_type),
        // );
        if let Some(range) = filter.assets_cap_range {
            let (min_cap, max_cap) = assets_cap_label_range(Some(range));

            criteria = criteria.gte("total_assets", min_cap);
            criteria = criteria.lte("total_assets", max_cap);
            // criteria.add_condition("total_assets", SearchOp::Gte, SearchValue::Int(min_cap));
            // criteria.add_condition("total_assets", SearchOp::Lte, SearchValue::Int(max_cap));
        }
        if let Some(signals) = filter.signals {
            criteria = criteria.gte("signals", signals);
            // criteria.add_condition("signals", SearchOp::All, SearchValue::Array(signals));
        }

        if let Some(cyield) = filter.r#yield
            && cyield > 0.0
        {
            let dec_yield: Decimal = Decimal::from_f32_retain(cyield).unwrap();
            let dec_yield = dec_yield / Decimal::from(100);

            criteria = criteria.gte("yield", dec_yield);
            // criteria.add_condition("yield", SearchOp::Gte, SearchValue::Decimal(dec_yield));
        }

        if let Some(limit) = filter.limit {
            // criteria.add_limit(limit);
            criteria = criteria.limit(limit);
        }

        // criteria.add_sort("total_assets", false);
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
