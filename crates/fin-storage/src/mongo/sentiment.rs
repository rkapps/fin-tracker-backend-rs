use async_trait::async_trait;
use fin_domain::ticker::TickerSentiment;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use storage_core::core::{Repository as _, search::{SearchCriteria, SearchOp, SearchValue}};

use crate::{mongo::MongoStorageService, service::TickerSentimentStorageService};
use anyhow::Result;

#[async_trait]
impl TickerSentimentStorageService for MongoStorageService {

    async fn get_ticker_sentiments(&self, symbol: &str) -> Result<Vec<TickerSentiment>> {
        let score = dec!(0);
        self.get_ticker_sentiments_with_score(symbol, &score).await
    }

    async fn get_ticker_sentiments_with_score(
        &self,
        symbol: &str,
        score: &Decimal,
    ) -> Result<Vec<TickerSentiment>> {
        let mut criteria = SearchCriteria::new();
        criteria.add_condition(
            "symbol",
            SearchOp::Eq,
            SearchValue::String(symbol.to_uppercase().to_string()),
        );
        criteria.add_condition(
            "relevance_score",
            SearchOp::Gte,
            SearchValue::Decimal(score.clone()),
        );

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
        sentiments: &Vec<TickerSentiment>,
    ) -> Result<()> {
        let Ok(repo) = self.manager.ticker_sentiments().await else {
            return Err(anyhow::anyhow!(format!(
                "Error saving TickerSentiment for '{}'",
                symbol
            )));
        };
        let mut repo = repo.lock().await;
        for sentiment in sentiments {
            repo.insert(sentiment.clone()).await?;
        }
        Ok(())
    }

}
