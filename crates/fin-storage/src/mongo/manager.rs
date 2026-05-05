use std::sync::Arc;

use anyhow::Result;
use fin_domain::tickers::{TICKER_ALPHA_COLLECTION_NAME, TICKER_NEWS_COLLECTION_NAME, TickerNews};
use fin_domain::tickers::{
    TICKER_COLLECTION_NAME, TICKER_CONTROL_COLLECTION_NAME, TICKER_EMBEDDING_COLLECTION_NAME,
    TICKER_HISTORY_COLLECTION_NAME, TICKER_INDICATOR_COLLECTION_NAME,
    TICKER_SENTIMENT_COLLECTION_NAME, Ticker, TickerAlpha, TickerControl, TickerEmbedding,
    TickerHistory, TickerIndicator, TickerSentiment,
};
use storage_core::mongo::{database::MongoDatabase, repository::MongoRepository};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct MongoStorageManager {
    db: MongoDatabase,
}

impl MongoStorageManager {
    pub async fn new(uri: &str, name: &str) -> Result<Self> {
        let mut mdb = MongoDatabase::new(uri, name).await?;
        mdb.register_collection::<String, Ticker>(TICKER_COLLECTION_NAME.to_string())
            .await?;
        mdb.register_collection::<String, TickerControl>(
            TICKER_CONTROL_COLLECTION_NAME.to_string(),
        )
        .await?;

        mdb.register_collection::<String, TickerHistory>(
            TICKER_HISTORY_COLLECTION_NAME.to_string(),
        )
        .await?;
        mdb.register_collection::<String, TickerIndicator>(
            TICKER_INDICATOR_COLLECTION_NAME.to_string(),
        )
        .await?;

        mdb.register_collection::<String, TickerSentiment>(
            TICKER_SENTIMENT_COLLECTION_NAME.to_string(),
        )
        .await?;
        mdb.register_collection::<String, TickerEmbedding>(
            TICKER_EMBEDDING_COLLECTION_NAME.to_string(),
        )
        .await?;

        mdb.register_collection::<String, TickerNews>(TICKER_NEWS_COLLECTION_NAME.to_string())
            .await?;

        mdb.register_collection::<String, TickerAlpha>(TICKER_ALPHA_COLLECTION_NAME.to_string())
            .await?;

        Ok(MongoStorageManager { db: mdb })
    }

    pub async fn tickers(&self) -> Result<Arc<Mutex<MongoRepository<String, Ticker>>>> {
        self.db
            .collection::<String, Ticker>(TICKER_COLLECTION_NAME.to_string())
            .await
    }

    pub async fn ticker_controls(
        &self,
    ) -> Result<Arc<Mutex<MongoRepository<String, TickerControl>>>> {
        self.db
            .collection::<String, TickerControl>(TICKER_CONTROL_COLLECTION_NAME.to_string())
            .await
    }

    pub async fn ticker_history(
        &self,
    ) -> Result<Arc<Mutex<MongoRepository<String, TickerHistory>>>> {
        self.db
            .collection::<String, TickerHistory>(TICKER_HISTORY_COLLECTION_NAME.to_string())
            .await
    }

    pub async fn ticker_indicators(
        &self,
    ) -> Result<Arc<Mutex<MongoRepository<String, TickerIndicator>>>> {
        self.db
            .collection::<String, TickerIndicator>(TICKER_INDICATOR_COLLECTION_NAME.to_string())
            .await
    }

    pub async fn ticker_sentiments(
        &self,
    ) -> Result<Arc<Mutex<MongoRepository<String, TickerSentiment>>>> {
        self.db
            .collection::<String, TickerSentiment>(TICKER_SENTIMENT_COLLECTION_NAME.to_string())
            .await
    }

    pub async fn ticker_embeddings(
        &self,
    ) -> Result<Arc<Mutex<MongoRepository<String, TickerEmbedding>>>> {
        self.db
            .collection::<String, TickerEmbedding>(TICKER_EMBEDDING_COLLECTION_NAME.to_string())
            .await
    }
    pub async fn ticker_news(&self) -> Result<Arc<Mutex<MongoRepository<String, TickerNews>>>> {
        self.db
            .collection::<String, TickerNews>(TICKER_NEWS_COLLECTION_NAME.to_string())
            .await
    }

    pub async fn ticker_alphas(&self) -> Result<Arc<Mutex<MongoRepository<String, TickerAlpha>>>> {
        self.db
            .collection::<String, TickerAlpha>(TICKER_ALPHA_COLLECTION_NAME.to_string())
            .await
    }
}
