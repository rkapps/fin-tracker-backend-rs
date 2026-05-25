use anyhow::Result;
use fin_core::tickers::{search::search_tickers, sentiments::search_ticker_sentiments};
use fin_domain::{
    dto::{
        ticker_chart_entity::TickerChartEntity, ticker_entity::TickerEntity,
        ticker_group::TickerGroup, ticker_news_entity::TickerNewsEntity, ticker_peer::TickerPeer,
        ticker_search_param::TickerSearchParam, ticker_sentiment_entity::TickerSentimentEntity,
    },
    tickers::{Ticker, TickerFilter, TickerIndicator},
    utils::data_utils::get_overview_embeddings,
};
use fin_storage::service::StorageService;
use rust_decimal::{Decimal, prelude::ToPrimitive};
use rustic_ml::{EmbeddingClient, search};
use std::{collections::HashMap, fmt::Debug, sync::Arc};
use tracing::debug;

#[derive(Debug, Clone)]
pub struct TickersService {
    pub storage_service: Arc<dyn StorageService>,
    pub embedding_client: Arc<dyn EmbeddingClient>,
}

impl TickersService {
    pub fn new(
        storage_service: Arc<dyn StorageService>,
        embedding_client: Arc<dyn EmbeddingClient>,
    ) -> TickersService {
        TickersService {
            storage_service,
            embedding_client,
        }
    }

    pub async fn get_ticker_peers_for_symbols(
        &self,
        symbols: Vec<String>,
        limit: usize,
    ) -> Result<Vec<TickerPeer>> {
        self.storage_service
            .get_ticker_peers_by_symbols(symbols, limit)
            .await
            .map_err(|e| anyhow::anyhow!(format!("Get Ticker Groups error: {}", e)))
    }

    pub async fn get_ticker_groups(&self) -> Result<Vec<TickerGroup>> {
        self.storage_service
            .get_ticker_groups()
            .await
            .map_err(|e| anyhow::anyhow!(format!("Get Ticker Groups error: {}", e)))
    }

    pub async fn get_ticker_charts(&self, symbol: &str) -> Result<Vec<TickerChartEntity>> {
        let indicators = self
            .storage_service
            .get_ticker_indicators(symbol)
            .await
            .map_err(|e| anyhow::anyhow!(format!("Get Ticker error: {}", e)))?;

        let indicator_map: HashMap<String, TickerIndicator> = indicators
            .iter()
            .map(|t| (t.id.clone(), t.clone()))
            .collect();

        let history = self
            .storage_service
            .get_ticker_history(symbol)
            .await
            .map_err(|e| anyhow::anyhow!(format!("Get Ticker error: {}", e)))?;

        let charts = history
            .into_iter()
            .filter_map(|b| {
                indicator_map.get(&b.id).map(|val_a| {
                    let sma_50 = val_a
                        .values
                        .get("sma_50")
                        .unwrap_or(&Decimal::ZERO)
                        .to_f64()
                        .unwrap_or_default();

                    let sma_200 = val_a
                        .values
                        .get("sma_200")
                        .unwrap_or(&Decimal::ZERO)
                        .to_f64()
                        .unwrap_or_default();

                    TickerChartEntity {
                        symbol: b.metadata.symbol,
                        date: b.date,
                        close: b.close.to_f64().unwrap_or_default(),
                        sma_50,
                        sma_200,
                    }
                })
            })
            .collect();

        Ok(charts)
    }

    pub async fn get_ticker_news(&self, symbol: &str) -> Result<Vec<TickerNewsEntity>> {
        let news = self
            .storage_service
            .get_ticker_news(symbol)
            .await
            .map_err(|e| anyhow::anyhow!(format!("Get Ticker Groups error: {}", e)))?;

        debug!("Ticker {} news: {}", symbol, news.len());
        let news_entity: Vec<TickerNewsEntity> = news
            .iter()
            .map(|n| {
                let entity = n.clone();
                TickerNewsEntity {
                    date: entity.date,
                    description: entity.description,
                    source: entity.source,
                    symbol: entity.symbol,
                    title: entity.title,
                    url: entity.url,
                }
            })
            .collect();
        Ok(news_entity)
    }

    pub async fn search_tickers(&self, param: TickerSearchParam) -> Result<Vec<TickerEntity>> {
        search_tickers(
            self.storage_service.clone(),
            self.embedding_client.clone(),
            param,
        )
        .await
    }

    pub async fn search_ticker_sentiments(
        &self,
        param: TickerSearchParam,
    ) -> Result<Vec<TickerSentimentEntity>> {
        if let Some(symbols) = param.symbols
            && let Some(query) = param.query
        {
            let list: Vec<String> = symbols.split(',').map(|s| s.to_string()).collect();
            let limit = param.limit.unwrap_or(20);
            search_ticker_sentiments(
                self.storage_service.clone(),
                self.embedding_client.clone(),
                list,
                query,
                limit,
            )
            .await
        } else {
            Ok(Vec::new())
        }
    }
}
