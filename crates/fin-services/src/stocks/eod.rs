use std::{collections::HashMap, time::Duration};

use crate::stocks::StocksService;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use tracing::{error, info};

impl StocksService {
    // update_tickers_eod updates eod ticker information like price, technicals
    // pub async fn handle_tickers_eod(&self) -> Result<()> {
    //     let tickers = self.storage_service.get_tickers_by_marketcap().await?;
    //     let symbols = tickers.iter().map(|t| t.symbol.clone());
    //     let delay = Duration::from_millis(300); // Sleep for 300 milliseconds

    //     // Loop through the symbols
    //     // Get ticker
    //     // update ticker
    //     // update ticker history
    //     // update technicals
    //     let length = tickers.len();
    //     for (i, symbol) in symbols.enumerate() {
    //         let mut tc = self.storage_service.get_ticker_control(&symbol).await?;
    //         let mut ticker = self.storage_service.get_ticker(&symbol).await?;
    //         if i%20 == 0 {
    //             info!("Updating Ticker: {} {}/{}", ticker.symbol, i+1, length);
    //         }
    //         if let Err(e) = self
    //             .update_and_save_single_ticker(&mut tc, &mut ticker)
    //             .await
    //         {
    //             error!("Ticker {}: {}", symbol, e);
    //             continue;
    //         }
    //         sleep(delay).await;
    //         // break;
    //     }
    //     Ok(())
    // }

    pub async fn handle_tickers_eod(&self) -> Result<()> {
        let tickers = self.storage_service.get_tickers_by_marketcap().await?;
        let total = tickers.len();

        // Limit to 5 concurrent requests (matches rate limit)
        let semaphore = Arc::new(Semaphore::new(5));
        let delay = Duration::from_millis(200); // Slightly faster since we have 5 concurrent

        info!("Processing {} tickers with 5 concurrent workers", total);

        let tasks: Vec<_> = tickers
            .into_iter()
            .enumerate()
            .map(|(i, ticker)| {
                let sem = semaphore.clone();
                let storage = self.storage_service.clone(); // ← Check: Does this work?
                let symbol = ticker.symbol.clone();
                // Recreate a StocksService with the cloned pieces
                let service = StocksService {
                    storage_service: self.storage_service.clone(),
                    provider_service: self.provider_service.clone(),
                    embedding_client: self.embedding_client.clone(),
                    ml_service: self.ml_service.clone(),
                };

                tokio::spawn(async move {
                    // Acquire permit (waits if 5 tasks already running)
                    let _permit = sem.acquire().await.unwrap();

                    if i % 20 == 0 {
                        info!("Updating Ticker: {} {}/{}", symbol, i + 1, total);
                    }

                    // Get ticker data
                    let mut tc = match storage.get_ticker_control(&symbol).await {
                        Ok(tc) => tc,
                        Err(e) => {
                            error!("Failed to get ticker control for {}: {}", symbol, e);
                            return Err(e);
                        }
                    };

                    let mut ticker = match storage.get_ticker(&symbol).await {
                        Ok(t) => t,
                        Err(e) => {
                            error!("Failed to get ticker {}: {}", symbol, e);
                            return Err(e);
                        }
                    };

                    // Update and save
                    if let Err(e) = service
                        .update_and_save_single_ticker(&mut tc, &mut ticker)
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

    pub async fn handle_ticker_embeddings_eod(&self) -> Result<()> {
        let tickers = self.storage_service.get_tickers().await?;
        let symbols = tickers.iter().map(|t| t.symbol.clone());

        let length = tickers.len();
        for (i, symbol) in symbols.enumerate() {
            let mut ticker = self.storage_service.get_ticker(&symbol).await?;
            if i % 20 == 0 {
                info!("Updating Ticker: {} {}/{}", ticker.symbol, i + 1, length);
            }
            if let Err(e) = self.update_single_ticker_embedding(&mut ticker).await {
                error!("Ticker overview embedding {}: {}", symbol, e);
                continue;
            } // break;
        }

        Ok(())
    }

    pub async fn handle_ticker_prediction_signals_eod(&self) -> Result<()> {
        let tickers = self.storage_service.get_tickers_by_marketcap().await?;
        let symbols = tickers.iter().map(|t| t.symbol.clone());

        let length = tickers.len();
        let mut sasm = HashMap::new();

        for (i, symbol) in symbols.enumerate() {
            let mut ticker = self.storage_service.get_ticker(&symbol).await?;
            // if !(ticker.symbol == "NVDA" || ticker.symbol == "AAPL") {
            //     continue;
            // }

            if i % 20 == 0 {
                info!("Updating Ticker: {} {}/{}", ticker.symbol, i + 1, length);
            }

            let sector = ticker.sector.clone().unwrap();
            if !sasm.contains_key(&sector) {
                match self.storage_service.get_ticker_alphas_by_key(&sector).await {
                    Ok(c) => {
                        sasm.insert(sector.clone(), c);
                    }
                    Err(e) => {
                        error!("Error getting SectorAlphas for {}-{}", ticker.symbol, e);
                        sasm.insert(sector.clone(), Vec::new());
                    }
                };
            }

            let sas = sasm.get(&sector).unwrap();

            if let Err(e) = self
                .update_single_ticker_prediction_signals(&mut ticker, sas)
                .await
            {
                error!("Ticker Prediction error {}: {}", symbol, e);
                continue;
            }

            // save the predictions
            self.storage_service.save_ticker(ticker).await?;
            
        }

        Ok(())
    }
}
