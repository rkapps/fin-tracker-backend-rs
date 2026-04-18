use crate::{
    HttpClient, alpha::{
        self,
        model::{AlphaEtf, AlphaTicker, AlphaTickerSentimentFeed},
    }, cmc::{self, model::CmcCryptoData}, tiingo::{
        self,
        api::get_stock_etf_realtime,
        model::{TiingoTickerHistory, TiingoTickerRealtime},
    }
};
use anyhow::Result;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct ProviderService {
    http_client: HttpClient,
    alpha_key: String,
    tiingo_token: String,
    coinmarketcap_key: String,
}

impl ProviderService {
    pub fn new(alpha_key: &str, tiingo_token: &str, coinmarketcap_key: &str) -> Result<Self> {
        let http_client = HttpClient::new().expect("Http Client cannot be configured.");

        Ok(ProviderService {
            http_client,
            alpha_key: alpha_key.to_string(),
            tiingo_token: tiingo_token.to_string(),
            coinmarketcap_key: coinmarketcap_key.to_string()
        })
    }

    pub async fn get_stock_etf_realtime(&self, symbol: &str) -> Result<TiingoTickerRealtime> {
        let realtime = get_stock_etf_realtime(&self.http_client, symbol, &self.tiingo_token).await?;
        Ok(realtime)
    }


    pub async fn get_stock(&self, symbol: &str) -> Result<AlphaTicker> {
        let raw = alpha::api::get_stock(&self.http_client, symbol, &self.alpha_key).await?;
        Ok(raw)
    }


    pub async fn get_etf(&self, symbol: &str) -> Result<AlphaEtf> {
        let raw = alpha::api::get_etf(&self.http_client, symbol, &self.alpha_key).await?;
        Ok(raw)
    }

    pub async fn get_crypto(&self, symbols: Vec<String>) -> Result<CmcCryptoData> {
        let raw = cmc::api::get_crypto(&self.http_client, symbols, &self.coinmarketcap_key).await?;
        Ok(raw)
    }

    pub async fn get_ticker_sentiment(
        &self,
        symbol: &str,
        date_from: &DateTime<Utc>,
    ) -> Result<Vec<AlphaTickerSentimentFeed>> {
        let feeds =
            alpha::api::get_stock_sentiments(&self.http_client, symbol, &self.alpha_key, date_from)
                .await?;
        Ok(feeds)
    }

    pub async fn get_stock_history(
        &self,
        symbol: &str,
        start_date: &DateTime<Utc>,
    ) -> Result<Vec<TiingoTickerHistory>> {
        tiingo::api::get_stock_history(&self.http_client, symbol, &self.tiingo_token, start_date)
            .await
    }

    pub async fn get_crypto_history(
        &self,
        symbol: &str,
        start_date: &DateTime<Utc>,
        frequency: &str,
    ) -> Result<Vec<TiingoTickerHistory>> {
        tiingo::api::get_crypto_history(
            &self.http_client,
            symbol,
            &self.tiingo_token,
            start_date,
            frequency,
        )
        .await
    }
}
