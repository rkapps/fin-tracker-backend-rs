use std::{collections::HashMap, fmt::Debug};

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fin_domain::{
    dto::screen_param::TickerScreenParam,
    ticker::{
        IndicatorWindow, Ticker, TickerAlpha, TickerControl, TickerEmbedding, TickerHistory,
        TickerIndicator, TickerSentiment,
    },
};
use rust_decimal::Decimal;
// use std::collections::HashMap;

#[async_trait]
pub trait StorageService:
    TickerControlStorageService
    + TickerStorageService
    + TickerHistoryStorageService
    + TickerIndicatorStorageService
    + TickerSentimentStorageService
    + TickerEmbeddingStorageService
    + TickerAlphaStorageService
    + Send
    + Sync
    + Debug
{
}

#[async_trait]
pub trait TickerControlStorageService: Send + Sync + Debug {
    async fn get_ticker_control(&self, symbol: &str) -> Result<TickerControl>;
    async fn save_ticker_control(&self, tc: TickerControl) -> Result<()>;
}

#[async_trait]
pub trait TickerStorageService: Send + Sync + Debug {
    async fn get_ticker_by_symbol(&self, symbol: &str) -> Result<Ticker>;
    async fn get_ticker_groups(&self) -> Result<HashMap<String, Vec<String>>>;
    async fn get_ticker_peers_by_industry(&self, symbol: &str) -> Result<Vec<Ticker>>;
    async fn get_ticker_peers_by_sector(&self, symbol: &str) -> Result<Vec<Ticker>>;
    async fn get_tickers(&self) -> Result<Vec<Ticker>>;
    async fn get_tickers_by_symbols(&self, symbols: Vec<String>) -> Result<Vec<Ticker>>;
    async fn get_tickers_by_movers(&self, function: &str) -> Result<Vec<Ticker>>;
    async fn get_ticker_by_sector(&self, sector: &str) -> Result<Vec<Ticker>>;
    async fn get_tickers_by_marketcap(&self) -> Result<Vec<Ticker>>;
    async fn get_ticker_industry_embeddings(&self) -> Result<Vec<(Ticker, Vec<f32>)>>;
    async fn get_ticker_overview_embeddings(&self) -> Result<Vec<(Ticker, Vec<f32>)>>;

    async fn search_tickers(&self, param: TickerScreenParam) -> Result<Vec<Ticker>>;

    async fn save_ticker(&self, ticker: Ticker) -> Result<()>;
}

#[async_trait]
pub trait TickerHistoryStorageService: Send + Sync + Debug {

    async fn delete_ticker_history(&self, symbol: &str) -> Result<()>;
    async fn get_ticker_history(&self, symbol: &str) -> Result<Vec<TickerHistory>>;
    async fn get_ticker_history_by_date(
        &self,
        symbol: &str,
        from_date: DateTime<Utc>,
    ) -> Result<Vec<TickerHistory>>;
    async fn get_ticker_history_latest(&self, symbol: &str) -> Result<Vec<TickerHistory>>;
    async fn save_ticker_history(&self, symbol: &str, hist: &Vec<TickerHistory>) -> Result<()>;
}

#[async_trait]
pub trait TickerIndicatorStorageService: Send + Sync + Debug {

    async fn delete_ticker_indicators(&self, symbol: &str) -> Result<()>;
    async fn get_ticker_indicators(&self, symbol: &str) -> Result<Vec<TickerIndicator>>;
    async fn get_ticker_indicators_latest(&self, symbol: &str) -> Result<TickerIndicator>;

    async fn get_ticker_indicators_by_symbol(
        &self,
        symbol: &str,
        from_date: DateTime<Utc>,
    ) -> Result<Vec<TickerIndicator>>;
    async fn get_ticker_indicators_last_n(
        &self,
        symbol: &str,
        n: usize,
    ) -> Result<Vec<TickerIndicator>>;
    async fn get_ticker_indicators_map_by_sector(
        &self,
        sector: &str,
        from_date: DateTime<Utc>,
    ) -> Result<HashMap<String, Vec<TickerIndicator>>>;

    async fn get_ticker_indicators_window(&self, symbol: &str) -> Result<IndicatorWindow>;

    async fn save_ticker_indicators(
        &self,
        symbol: &str,
        indicators: &Vec<TickerIndicator>,
    ) -> Result<()>;
}

#[async_trait]
pub trait TickerSentimentStorageService: Send + Sync + Debug {
    async fn get_ticker_sentiments(&self, symbol: &str) -> Result<Vec<TickerSentiment>>;
    async fn get_ticker_sentiments_with_score(
        &self,
        symbol: &str,
        score: &Decimal,
    ) -> Result<Vec<TickerSentiment>>;

    async fn save_ticker_sentiments(
        &self,
        symbol: &str,
        sentiments: &Vec<TickerSentiment>,
    ) -> Result<()>;
}

#[async_trait]
pub trait TickerEmbeddingStorageService: Send + Sync + Debug {
    async fn get_ticker_embeddings(&self, symbol: &str) -> Result<Vec<TickerEmbedding>>;
    async fn save_ticker_embeddings(
        &self,
        symbol: &str,
        sentiments: &Vec<TickerEmbedding>,
    ) -> Result<()>;
}

#[async_trait]
pub trait TickerAlphaStorageService: Send + Sync + Debug {
    async fn get_ticker_alphas_by_key(&self, key: &str) -> Result<Vec<TickerAlpha>>;
    async fn save_ticker_alphas(&self, sas: &Vec<TickerAlpha>) -> Result<()>;
}
