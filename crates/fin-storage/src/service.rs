use std::fmt::Debug;

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fin_domain::{dto::screen_param::TickerScreenParam, ticker::{
    IndicatorWindow, Ticker, TickerControl, TickerEmbedding, TickerHistory, TickerIndicator, TickerSentiment
}};
use rust_decimal::Decimal;
// use std::collections::HashMap;

#[async_trait]
pub trait StorageService: Send + Sync + Debug{
    async fn get_ticker_control(&self, symbol: &str) -> Result<TickerControl>;
    async fn get_ticker(&self, symbol: &str) -> Result<Ticker>;
    async fn get_ticker_peers_by_industry(&self, symbol: &str) -> Result<Vec<Ticker>>;
    async fn get_ticker_peers_by_sector(&self, symbol: &str) -> Result<Vec<Ticker>>;    
    async fn get_tickers(&self) -> Result<Vec<Ticker>>;
    async fn search_tickers(
        &self,
        param: TickerScreenParam,
    ) -> Result<Vec<Ticker>>;


    async fn get_ticker_history(&self, symbol: &str) -> Result<Vec<TickerHistory>>;
    async fn get_ticker_history_by_date(&self, symbol: &str, from_date: DateTime<Utc>) -> Result<Vec<TickerHistory>>;
    async fn get_ticker_history_latest(&self, symbol: &str ) -> Result<Vec<TickerHistory>>;
    async fn get_ticker_indicators(&self, symbol: &str) -> Result<Vec<TickerIndicator>>;
    async fn get_ticker_indicators_latest(&self, symbol: &str) -> Result<Vec<TickerIndicator>>;
    async fn get_ticker_indicators_last_two(&self, symbol: &str) -> Result<Vec<TickerIndicator>>;
    async fn get_ticker_indicators_window(&self, symbol: &str) -> Result<IndicatorWindow>;


    async fn get_ticker_industry_embeddings(&self) -> Result<Vec<(Ticker, Vec<f32>)>>;
    async fn get_ticker_overview_embeddings(&self) -> Result<Vec<(Ticker, Vec<f32>)>>;
    async fn get_ticker_sentiments(&self, symbol: &str) -> Result<Vec<TickerSentiment>>;
    async fn get_ticker_sentiments_with_score(
        &self,
        symbol: &str,
        score: &Decimal,
    ) -> Result<Vec<TickerSentiment>>;

    async fn get_ticker_embeddings(&self, symbol: &str) -> Result<Vec<TickerEmbedding>>;
    // async fn get_all_ticker_embeddings(&self) -> HashMap<String, Vec<TickerEmbedding>>;
    async fn save_ticker_control(&self, tc: TickerControl) -> Result<()>;

    async fn save_ticker(&self, ticker: Ticker) -> Result<()>;
    async fn save_ticker_history(&self, symbol: &str, hist: &Vec<TickerHistory>) -> Result<()>;
    async fn save_ticker_indicators(
        &self,
        symbol: &str,
        indicators: &Vec<TickerIndicator>,
    ) -> Result<()>;
     
    async fn save_ticker_sentiments(
        &self,
        symbol: &str,
        sentiments: &Vec<TickerSentiment>,
    ) -> Result<()>;

    async fn save_ticker_embeddings(
        &self,
        symbol: &str,
        sentiments: &Vec<TickerEmbedding>,
    ) -> Result<()>;


            
}
