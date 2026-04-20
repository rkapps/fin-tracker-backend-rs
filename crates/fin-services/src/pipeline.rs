use agentic_core::client::embeddings::EmbeddingClient;
use anyhow::Result;
use fin_core::tickers::update::{
    update_all_tickers, update_ticker_prediction_signals, update_ticker_realtime,
};
use fin_domain::tickers::{AssetType, Ticker, TickerAlpha};
use fin_providers::ProviderService;
use fin_storage::service::StorageService;
use std::{collections::HashMap, sync::Arc};
use tokio::{sync::Semaphore, task::JoinHandle};
use tracing::{debug, error, info};

#[derive(Debug)]
pub struct PipeLineService {
    pub storage_service: Arc<dyn StorageService>,
    pub provider_service: ProviderService,
    pub embedding_client: Arc<dyn EmbeddingClient>,
}

impl PipeLineService {
    pub fn new(
        storage_service: Arc<dyn StorageService>,
        provider_service: ProviderService,
        embedding_client: Arc<dyn EmbeddingClient>,        
    ) -> PipeLineService {
        PipeLineService {
            storage_service,
            provider_service,
            embedding_client
        }
    }

    pub async fn update_tickers_eod(&self, symbols: &str) -> Result<()> {
        let all_tickers = if !symbols.is_empty() {
            let list: Vec<String> = symbols.split(',').map(|s| s.to_string()).collect();
            debug!("List: {:?}", list);
            self.storage_service.get_tickers_by_symbols(list).await?
        } else {
            self.storage_service.get_tickers_by_marketcap().await?
        };

        let all_controls = self.storage_service.get_ticker_controls().await?;

        update_all_tickers(
            self.storage_service.clone(),
            self.provider_service.clone(),
            self.embedding_client.clone(),
            all_controls,
            all_tickers,
        )
        .await?;

        Ok(())
    }

    pub async fn update_ticker_eod_prediction_signals(&self, symbols: &str) -> Result<()> {
        let all_tickers = if !symbols.is_empty() {
            let list: Vec<String> = symbols.split(',').map(|s| s.to_string()).collect();
            debug!("List: {:?}", list);
            self.storage_service.get_tickers_by_symbols(list).await?
        } else {
            self.storage_service.get_tickers_by_marketcap().await?
        };
        // let symbols = tickers.iter().map(|t| t.symbol.clone());

        let length = all_tickers.len();
        let mut sasm = HashMap::new();
        info!("Updating Ticker Predictions: {} ", length);
        let semaphore = Arc::new(Semaphore::new(3));

        for ticker in &all_tickers {
            let sector = ticker.sector.clone().unwrap();
            if !sasm.contains_key(&sector) {
                match self.storage_service.get_ticker_alphas_by_key(&sector).await {
                    Ok(c) => {
                        sasm.insert(sector.clone(), c);
                    }
                    Err(e) => {
                        error!(
                            "       Error getting SectorAlphas for {}-{}",
                            ticker.symbol, e
                        );
                        sasm.insert(sector.clone(), Vec::new());
                    }
                };
            }
        }

        let tasks: Vec<JoinHandle<Result<Ticker, anyhow::Error>>> = all_tickers
            .into_iter()
            .enumerate()
            .map(|(i, ticker)| {
                let sem = semaphore.clone();
                let nsasm = sasm.clone();
                let storage_service = self.storage_service.clone();

                tokio::spawn(async move {
                    let _permit = sem.acquire().await.unwrap();
                    let mut ticker = ticker;

                    if i % 20 == 0 {
                        info!(
                            "Updating Ticker Predictions: {} {}/{}",
                            ticker.symbol,
                            i + 1,
                            length
                        );
                    }
                    let sas: Vec<TickerAlpha> = if let Some(sector) = ticker.sector.clone() {
                        nsasm.get(&sector).unwrap().to_vec()
                    } else {
                        Vec::new()
                    };

                    if let Err(e) =
                        update_ticker_prediction_signals(storage_service.clone(), &mut ticker, &sas)
                            .await
                    {
                        error!("       Prediction error {}: {}", ticker.symbol, e);
                        // continue;
                    }

                    Ok(ticker)
                })
            })
            .collect();

        let results = futures::future::join_all(tasks).await;

        let mut updated_tickers = Vec::new();
        let mut success = 0;
        let mut failed = 0;

        for result in results {
            match result {
                Ok(Ok(ticker)) => {
                    success += 1;
                    updated_tickers.push(ticker);
                }
                Ok(Err(e)) => {
                    error!("Ticker update failed: {}", e);
                    failed += 1;
                }
                Err(e) => {
                    error!("Task panicked: {}", e);
                    failed += 1;
                }
            }
        }

        // bulk write at the end
        if !updated_tickers.is_empty() {
            self.storage_service.save_tickers(updated_tickers).await?;
        }

        info!("Completed: {} successful, {} failed", success, failed);
        Ok(())
    }

    pub async fn update_realtime_stocks_etfs(&self) -> Result<()> {
        let mut all_tickers = self.storage_service.get_tickers_by_marketcap().await?;
        all_tickers.retain(|t| t.asset_type == AssetType::Stock || t.asset_type == AssetType::Etf);
        let length = all_tickers.len();

        let mut updated_tickers = Vec::new();

        for (i, mut ticker) in all_tickers.into_iter().enumerate() {
            if i % 20 == 0 {
                info!(
                    "Updating Ticker Realtime: {} {}/{}",
                    ticker.symbol,
                    i + 1,
                    length
                );
            }
            match update_ticker_realtime(self.provider_service.clone(), &mut ticker).await {
                Ok(_) => updated_tickers.push(ticker),
                Err(e) => error!("Ticker Realtime error {}: {}", ticker.symbol, e),
            }
        }

        info!(
            "Realtime update complete: {}/{} updated",
            updated_tickers.len(),
            length
        );

        // bulk write at the end
        if !updated_tickers.is_empty() {
            self.storage_service.save_tickers(updated_tickers).await?;
        }

        Ok(())
    }

    pub async fn update_realtime_crypto(&self) -> Result<()> {
        Ok(())
    }
}
