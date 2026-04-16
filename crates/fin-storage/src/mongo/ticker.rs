use std::collections::HashMap;

use crate::{mongo::service::MongoStorageService, service::TickerStorageService};
use anyhow::Result;
use async_trait::async_trait;
use fin_domain::{
    dto::screen_param::TickerScreenParam,
    tickers::Ticker,
    utils::data_utils::{market_cap_label_range, market_cap_range},
};
use rust_decimal::Decimal;
use storage_core::core::{
    Repository as _,
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
            Err(e) => {
                Err(anyhow::anyhow!("Error getting Ticker: {}", e))
            }
        }
    }

    async fn get_tickers(&self) -> Result<Vec<Ticker>> {
        let mut criteria = SearchCriteria::new();
        criteria.add_sort("symbol", true);
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_tickers_by_symbols(&self, symbols: Vec<String>) -> Result<Vec<Ticker>> {
        let mut criteria = SearchCriteria::new();
        criteria.add_condition("symbol", SearchOp::In, SearchValue::Array(symbols));
        criteria.add_sort("symbol", true);
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
        let mut criteria = SearchCriteria::new();
        criteria.add_condition(
            "sector",
            SearchOp::Eq,
            SearchValue::String(sector.to_string()),
        );
        criteria.add_sort("market_cap", false);
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_tickers_by_marketcap(&self) -> Result<Vec<Ticker>> {
        let mut criteria = SearchCriteria::new();
        criteria.add_sort("market_cap", false);
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_tickers_by_movers(&self, function: &str) -> Result<Vec<Ticker>> {
        let mut criteria = SearchCriteria::new();
        match function {
            "top_gainers" => {
                criteria.add_sort("pr_diff_perc", false);
                criteria.add_limit(20);
            }
            "top_gainers_ytd" => {
                criteria.add_sort("performance_search.Ytd.perc", false);
                criteria.add_limit(20);
            }

            "top_losers" => {
                criteria.add_sort("pr_diff_perc", true);
                criteria.add_limit(20);
            }
            "top_losers_ytd" => {
                criteria.add_sort("performance_search.Ytd.perc", true);
                criteria.add_limit(20);
            }
            _ => {}
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
        let (min_cap, max_cap) = market_cap_range(ticker.market_cap);

        let mut criteria = SearchCriteria::new();
        criteria.add_condition("industry", SearchOp::Eq, SearchValue::String(industry));
        criteria.add_condition("market_cap", SearchOp::Gte, SearchValue::Int(min_cap));
        criteria.add_condition("market_cap", SearchOp::Lte, SearchValue::Int(max_cap));
        criteria.add_sort("market_cap", false);
        self.get_ticker_by_criteria(&criteria).await
    }

    async fn get_ticker_peers_by_sector(&self, symbol: &str) -> Result<Vec<Ticker>> {
        let ticker = self.get_ticker_by_symbol(symbol).await?;
        let sector = ticker.sector.unwrap_or_default();
        if sector.is_empty() {
            return Ok(Vec::new());
        }
        // Get reference ticker market cap bucket
        let (min_cap, max_cap) = market_cap_range(ticker.market_cap);

        let mut criteria = SearchCriteria::new();
        criteria.add_condition("sector", SearchOp::Eq, SearchValue::String(sector));
        criteria.add_condition("market_cap", SearchOp::Gte, SearchValue::Int(min_cap));
        criteria.add_condition("market_cap", SearchOp::Lte, SearchValue::Int(max_cap));
        criteria.add_sort("market_cap", false);

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

    async fn search_tickers(&self, param: TickerScreenParam) -> Result<Vec<Ticker>> {
        let mut criteria = SearchCriteria::new();
        if let Some(industry) = param.industry {
            criteria.add_condition(
                "industry",
                SearchOp::Contains,
                SearchValue::String(industry),
            );
        }

        let new_asset_type = param
            .asset_type
            .unwrap_or_else(|| "stock".to_string())
            .to_uppercase();
        criteria.add_condition(
            "asset_type",
            SearchOp::Eq,
            SearchValue::String(new_asset_type),
        );
        if let Some(range) = param.market_cap_range {
            let (min_cap, max_cap) = market_cap_label_range(Some(range));
            criteria.add_condition("market_cap", SearchOp::Gte, SearchValue::Int(min_cap));
            criteria.add_condition("market_cap", SearchOp::Lte, SearchValue::Int(max_cap));
        }
        if let Some(signals) = param.signals {
            criteria.add_condition("signals", SearchOp::All, SearchValue::Array(signals));
        }

        if let Some(cyield) = param.r#yield
            && cyield > 0.0
        {
            let dec_yield: Decimal = Decimal::from_f32_retain(cyield).unwrap();
            let dec_yield = dec_yield / Decimal::from(100);
            criteria.add_condition("yield", SearchOp::Gte, SearchValue::Decimal(dec_yield));
        }

        if let Some(limit) = param.limit {
            criteria.add_limit(limit);
        }

        criteria.add_sort("market_cap", false);
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
}
