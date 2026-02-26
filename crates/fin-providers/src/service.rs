use crate::{
    alpha::{
        self,
        model::{AlphaTicker, AlphaTickerSentimentFeed},
    },
    tiingo::{self, model::TiingoTickerHistory},
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use fin_http::HttpClient;

#[derive(Debug, Clone)]
pub struct ProviderService {
    http_client: HttpClient,
    alpha_key: String,
    tiingo_token: String,
}

impl ProviderService {
    pub fn new(http_client: HttpClient, alpha_key: &str, tiingo_token: &str) -> Self {
        ProviderService {
            http_client,
            alpha_key: alpha_key.to_string(),
            tiingo_token: tiingo_token.to_string(),
        }
    }

    pub async fn get_stock(&self, symbol: &str) -> Result<AlphaTicker> {
        let raw = alpha::api::get_stock(&self.http_client, symbol, &self.alpha_key).await?;
        // Ok(AlphaTicker::to_domain(raw))
        Ok(raw)
    }


    pub async fn get_ticker_sentiment(
        &self,
        symbol: &str,
        date_from: &DateTime<Utc>,
    ) -> Result<Vec<AlphaTickerSentimentFeed>> {
        let feeds = alpha::api::get_stock_sentiments(
            &self.http_client,
            symbol,
            &self.alpha_key,
            &date_from,
        )
        .await?;
        Ok(feeds)
    }

    pub async fn get_stock_history(
        &self,
        symbol: &str,
        start_date: &DateTime<Utc>,
    ) -> Result<Vec<TiingoTickerHistory>> {
        tiingo::api::get_stock_history(&self.http_client, symbol, &self.tiingo_token, &start_date)
            .await
    }

    pub async fn get_crypto_history(
        &self,
        symbol: &str,
        frequency: &str,
    ) -> Result<Vec<TiingoTickerHistory>> {
        tiingo::api::get_crypto_history(&self.http_client, symbol, frequency, &self.tiingo_token)
            .await
    }
}
