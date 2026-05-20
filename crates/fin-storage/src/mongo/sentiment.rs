use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fin_domain::tickers::TickerSentiment;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use rustic_storage::core::{repository::Repository, search::{SearchCriteria, SearchOp, SearchValue}};

use crate::{mongo::MongoStorageService, service::TickerSentimentStorageService};
use anyhow::Result;

#[async_trait]
impl TickerSentimentStorageService for MongoStorageService {
    async fn delete_ticker_sentiments_before(&self, date: DateTime<Utc>) -> Result<()> {
        let criteria = SearchCriteria::new().lt("date", date);
        // criteria.add_condition("date", SearchOp::Lt, SearchValue::DateTime(date));

        match self.manager.ticker_sentiments().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.delete_many(Some(criteria)).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error getting TickerSentiment: {}", e));
            }
        }
    }

    async fn get_ticker_sentiments(&self, symbol: &str) -> Result<Vec<TickerSentiment>> {
        let score = dec!(0);
        self.get_ticker_sentiments_with_score(symbol, &score).await
    }

    async fn get_ticker_sentiments_with_score(
        &self,
        symbol: &str,
        score: &Decimal,
    ) -> Result<Vec<TickerSentiment>> {
        let mut criteria = SearchCriteria::new().eq("symbol", symbol.to_uppercase()).gte("relevance_score", *score);
        // criteria.add_condition(
        //     "symbol",
        //     SearchOp::Eq,
        //     SearchValue::String(symbol.to_uppercase().to_string()),
        // );
        // criteria.add_condition(
        //     "relevance_score",
        //     SearchOp::Gte,
        //     SearchValue::Decimal(*score),
        // );

        match self.manager.ticker_sentiments().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.find(Some(criteria)).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error getting TickerSentiment: {}", e));
            }
        }
    }

    async fn save_ticker_sentiments(
        &self,
        symbol: &str,
        sentiments: Vec<TickerSentiment>,
    ) -> Result<()> {
        match self.manager.ticker_sentiments().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.bulk_update(sentiments).await
            }
            Err(e) => {
                return Err(anyhow::anyhow!(format!(
                    "Error saving TickerSentiments for {}: {}",
                    symbol, e
                )));
            }
        }
    }
}
