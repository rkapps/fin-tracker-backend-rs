use agentic_core::client::embeddings::EmbeddingClient;
use anyhow::Result;
use fin_core::tickers::update::{
    update_ticker, update_ticker_prediction_signals, update_ticker_realtime,
};
use fin_domain::tickers::AssetType;
use fin_providers::ProviderService;
use fin_storage::service::StorageService;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{sync::Semaphore, time::sleep};
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
            embedding_client,
        }
    }

    pub async fn update_tickers_eod(&self, symbols: &str) -> Result<()> {
                
        let tickers = if !symbols.is_empty() {
            let list: Vec<String> = symbols.split(',').map(|s| s.to_string()).collect();
            debug!("List: {:?}", list);
            self.storage_service.get_tickers_by_symbols(list).await?
        }  else {
            self.storage_service.get_tickers_by_marketcap().await?
        };
        let total = tickers.len();

        // Limit to 5 concurrent requests (matches rate limit)
        let semaphore = Arc::new(Semaphore::new(3));
        let delay = Duration::from_millis(1000); // Slightly faster since we have 5 concurrent

        info!("Processing {} tickers with 5 concurrent workers", total);

        let tasks: Vec<_> = tickers
            .into_iter()
            .enumerate()
            .map(|(i, ticker)| {
                let sem = semaphore.clone();
                let symbol = ticker.symbol.clone();
                let storage_service = self.storage_service.clone();
                let provider_service = self.provider_service.clone();
                let embedding_client = self.embedding_client.clone();

                tokio::spawn(async move {
                    // Acquire permit (waits if 5 tasks already running)
                    let _permit = sem.acquire().await.unwrap();

                    if i % 20 == 0 {
                        info!("Updating Ticker: {} {}/{}", symbol, i + 1, total);
                    }

                    // Get ticker data
                    let mut tc = match storage_service.get_ticker_control(&symbol).await {
                        Ok(tc) => tc,
                        Err(e) => {
                            error!("Failed to get ticker control for {}: {}", symbol, e);
                            return Err(e);
                        }
                    };

                    let mut ticker = match storage_service.get_ticker_by_symbol(&symbol).await {
                        Ok(t) => t,
                        Err(e) => {
                            error!("Failed to get ticker {}: {}", symbol, e);
                            return Err(e);
                        }
                    };

                    // Update and save
                    if let Err(e) = update_ticker(
                        storage_service,
                        provider_service,
                        embedding_client,
                        &mut tc,
                        &mut ticker,
                    )
                    .await
                    {
                        error!("Ticker {}: {}", symbol, e);
                        return Err(e);
                    }
                    // Rate limit delay
                    sleep(delay).await;

                    Ok::<_, anyhow::Error>(())
                })
            })
            .collect();

        // Wait for all tasks to complete
        let results = futures::future::join_all(tasks).await;

        // Count successes/failures
        let mut success = 0;
        let mut failed = 0;

        for result in results {
            match result {
                Ok(Ok(())) => success += 1,
                Ok(Err(_)) => failed += 1,
                Err(e) => {
                    error!("Task panicked: {}", e);
                    failed += 1;
                }
            }
        }

        info!("Completed: {} successful, {} failed", success, failed);

        Ok(())
    }

    pub async fn update_ticker_eod_prediction_signals(&self, symbols: &str) -> Result<()> {
        let tickers = if !symbols.is_empty() {
            let list: Vec<String> = symbols.split(',').map(|s| s.to_string()).collect();
            debug!("List: {:?}", list);
            self.storage_service.get_tickers_by_symbols(list).await?
        }  else {
            self.storage_service.get_tickers_by_marketcap().await?
        };
        let symbols = tickers.iter().map(|t| t.symbol.clone());

        let length = tickers.len();
        let mut sasm = HashMap::new();
        info!("Updating Ticker Predictions: {} ", length);

        for (i, symbol) in symbols.enumerate() {
            let mut ticker = self.storage_service.get_ticker_by_symbol(&symbol).await?;

            if i % 20 == 0 {
                info!(
                    "Updating Ticker Predictions: {} {}/{}",
                    ticker.symbol,
                    i + 1,
                    length
                );
            }

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

            let sas = sasm.get(&sector).unwrap();

            if let Err(e) =
                update_ticker_prediction_signals(self.storage_service.clone(), &mut ticker, sas)
                    .await
            {
                error!("       Prediction error {}: {}", symbol, e);
                continue;
            }

            // save the predictions
            self.storage_service.save_ticker(ticker).await?;
        }

        Ok(())
    }

    pub async fn update_realtime_stocks_etfs(&self) -> Result<()> {
        let mut tickers = self.storage_service.get_tickers_by_marketcap().await?;
        tickers.retain(|t| t.asset_type == AssetType::Stock || t.asset_type == AssetType::Etf);
        let symbols = tickers.iter().map(|t| t.symbol.clone());

        let length = tickers.len();

        for (i, symbol) in symbols.enumerate() {
            let mut ticker = self.storage_service.get_ticker_by_symbol(&symbol).await?;

            if i % 20 == 0 {
                info!(
                    "Updating Ticker Realtime: {} {}/{}",
                    ticker.symbol,
                    i + 1,
                    length
                );
            }
            if let Err(e) = update_ticker_realtime(
                self.storage_service.clone(),
                self.provider_service.clone(),
                &mut ticker,
            )
            .await
            {
                error!("Ticker Realtime error {}: {}", symbol, e);
                continue;
            }
            break;
        }
        Ok(())
    }

    pub async fn update_realtime_crypto(&self) -> Result<()> {
        Ok(())
    }
}
