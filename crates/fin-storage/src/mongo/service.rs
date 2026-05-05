use std::fmt::Debug;

use crate::{
    mongo::manager::MongoStorageManager,
    service::{
        StorageService, TickerAlphaStorageService, TickerControlStorageService,
        TickerEmbeddingStorageService, TickerHistoryStorageService, TickerIndicatorStorageService,
        TickerNewsStorageService, TickerSentimentStorageService, TickerStorageService,
    },
};
use anyhow::Result;
use fin_domain::tickers::{
    Ticker, TickerEmbedding, TickerHistory, TickerIndicator, TickerSentiment,
};
use storage_core::core::{Repository as _, search::SearchCriteria};

#[derive(Debug)]
pub struct MongoStorageService {
    pub manager: MongoStorageManager,
}
impl MongoStorageService {
    pub fn new(manager: MongoStorageManager) -> Self {
        Self { manager }
    }

    pub async fn get_ticker_by_criteria(&self, criteria: &SearchCriteria) -> Result<Vec<Ticker>> {
        match self.manager.tickers().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.find(Some(criteria.clone())).await
            }
            Err(e) => Err(anyhow::anyhow!("Error getting Ticker: {}", e)),
        }
    }

    pub async fn get_ticker_history_by_criteria(
        &self,
        criteria: &SearchCriteria,
    ) -> Result<Vec<TickerHistory>> {
        match self.manager.ticker_history().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.find(Some(criteria.clone())).await
            }
            Err(e) => Err(anyhow::anyhow!("Error getting TickerHistory: {}", e)),
        }
    }

    pub async fn get_ticker_indicators_by_criteria(
        &self,
        criteria: &SearchCriteria,
    ) -> Result<Vec<TickerIndicator>> {
        match self.manager.ticker_indicators().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.find(Some(criteria.clone())).await
            }
            Err(e) => Err(anyhow::anyhow!("Error getting TickerIndicator: {}", e)),
        }
    }

    pub async fn get_ticker_sentiments_by_criteria(
        &self,
        criteria: &SearchCriteria,
    ) -> Result<Vec<TickerSentiment>> {
        match self.manager.ticker_sentiments().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.find(Some(criteria.clone())).await
            }
            Err(e) => Err(anyhow::anyhow!("Error getting TickerSentiment: {}", e)),
        }
    }

    pub async fn get_ticker_embeddings_by_criteria(
        &self,
        criteria: &SearchCriteria,
    ) -> Result<Vec<TickerEmbedding>> {
        match self.manager.ticker_embeddings().await {
            Ok(repo) => {
                let mut repo = repo.lock().await;
                repo.find(Some(criteria.clone())).await
            }
            Err(e) => Err(anyhow::anyhow!("Error getting TickerEmbedding: {}", e)),
        }
    }
}

// 2. The Blanket Implementation (The "Glue")
impl<T> StorageService for T where
    T: TickerControlStorageService
        + TickerStorageService
        + TickerHistoryStorageService
        + TickerIndicatorStorageService
        + TickerSentimentStorageService
        + TickerEmbeddingStorageService
        + TickerNewsStorageService
        + TickerAlphaStorageService
        + Send
        + Sync
        + Debug
{
}
