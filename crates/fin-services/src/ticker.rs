use anyhow::Result;
use fin_domain::{
    dto::{
        ticker_chart_entity::TickerChartEntity, ticker_entity::TickerEntity,
        ticker_news_entity::TickerNewsEntity, ticker_search_param::TickerSearchParam,
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

    pub async fn get_ticker_groups(&self) -> Result<HashMap<String, Vec<String>>> {
        let groups = self
            .storage_service
            .get_ticker_groups()
            .await
            .map_err(|e| anyhow::anyhow!(format!("Get Ticker Groups error: {}", e)))?;
        Ok(groups)
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
        // let mut tickers = Vec::new();
        debug!("Search Param: {:#?}", param);
        let tickers: Vec<Ticker> = if let Some(symbols) = param.symbols {
            let list: Vec<String> = symbols.split(',').map(|s| s.to_string()).collect();
            debug!("List: {:?}", list);
            self.storage_service.get_tickers_by_symbols(list).await?
        } else if let Some(function) = param.function {
            match function.as_str() {
                "top_gainers" => {
                    self.storage_service
                        .get_tickers_by_top_gainers(param.asset_type)
                        .await?
                }
                "top_gainers_ytd" => {
                    self.storage_service
                        .get_tickers_by_top_gainers_ytd(param.asset_type)
                        .await?
                }
                "top_losers" => {
                    self.storage_service
                        .get_tickers_by_top_losers(param.asset_type)
                        .await?
                }
                "top_losers_ytd" => {
                    self.storage_service
                        .get_tickers_by_top_losers_ytd(param.asset_type)
                        .await?
                }
                _ => Vec::new(),
            }
        } else {
            let filter = TickerFilter::from(param.clone());
            let tickers = self
                .storage_service
                .search_tickers(filter)
                .await
                .map_err(|e| anyhow::anyhow!(format!("Get Ticker error: {}", e)))?;
            debug!("Tickers from storage: {}", tickers.len());
            self.search_tickers_by_overview_embedding(param.query, &tickers)
                .await?
        };

        debug!("Tickers: {}", tickers.len());
        let tentities = tickers
            .iter()
            .map(|t| TickerEntity::from(t.clone()))
            .collect();
        Ok(tentities)
    }

    pub async fn search_tickers_by_overview_embedding(
        &self,
        query: Option<String>,
        tickers: &[Ticker],
    ) -> Result<Vec<Ticker>> {
        let tickers = if let Some(query) = query
            && !tickers.is_empty()
        {
            let overview_candidates: Vec<(Ticker, Vec<f32>)> = get_overview_embeddings(tickers);
            debug!("Overview candidates: {}", overview_candidates.len());
            debug!("Query: {:?}", query);

            let candidates: Vec<(Ticker, Vec<f32>)> = get_overview_embeddings(tickers);
            let vectors = self.embedding_client.embed_text(&query).await?.into_vec();

            debug!(
                "Query vectors: {} candidates: {}",
                candidates.len(),
                candidates.len()
            );
            search(&vectors, &candidates, 1000)
                .into_iter()
                .filter_map(|(t, s)| {
                    // debug!("ticker: {}-{}", t.symbol, s);
                    if s > 0.25 { Some(t.clone()) } else { None }
                })
                .collect()
        } else {
            tickers.to_vec()
        };
        Ok(tickers)
    }
}
