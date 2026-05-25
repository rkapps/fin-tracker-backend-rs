use async_trait::async_trait;
use chrono::{DateTime, Utc};
use fin_domain::tickers::TickerSentiment;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use rustic_storage::core::{repository::Repository, search::SearchCriteria};

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

    async fn get_ticker_sentiments_by_ids(&self, ids: Vec<String>) -> Result<Vec<TickerSentiment>> {
        let criteria = SearchCriteria::new().in_values("id", ids);
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

    async fn get_ticker_sentiments(&self, symbol: &str) -> Result<Vec<TickerSentiment>> {
        let score = dec!(0);
        self.get_ticker_sentiments_with_score(vec![symbol.to_string()], &score)
            .await
    }

    async fn get_ticker_sentiments_with_score(
        &self,
        symbols: Vec<String>,
        score: &Decimal,
    ) -> Result<Vec<TickerSentiment>> {
        let criteria = SearchCriteria::new()
            .in_values("symbol", symbols)
            .gte("relevance_score", *score);
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
