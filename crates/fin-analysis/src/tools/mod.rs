use serde::Deserialize;

pub mod ticker_sentiment;
pub mod ticker_snapshot;
pub mod ticker_price_history;
pub mod ticker_indicator;
pub mod ticker_peers;
pub mod ticker_similarity;
pub mod ticker_screening;
pub mod ticker_taxonomy;




#[derive(Debug, Deserialize)]
pub struct TickerParam {
    symbol: String,
}