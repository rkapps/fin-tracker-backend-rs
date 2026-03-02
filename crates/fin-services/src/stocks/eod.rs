use std::time::Duration;

use crate::stocks::StocksService;
use anyhow::Result;
use tokio::time::sleep;
use tracing::{error, info};

impl StocksService {
    // update_tickers_eod updates eod ticker information like price, technicals
    pub async fn handle_tickers_eod(&self) -> Result<()> {
        let tickers = self.storage_service.get_tickers().await?;
        let symbols = tickers.iter().map(|t| t.symbol.clone());
        let delay = Duration::from_millis(300); // Sleep for 300 milliseconds

        // Loop through the symbols
        // Get ticker
        // update ticker
        // update ticker history
        // update technicals
        let length = tickers.len();
        for (i, symbol) in symbols.enumerate() {
            let mut tc = self.storage_service.get_ticker_control(&symbol).await?;
            let mut ticker = self.storage_service.get_ticker(&symbol).await?;
            if i%20 == 0 {
                info!("Updating Ticker: {} {}/{}", ticker.symbol, i+1, length);
            }
            if let Err(e) = self
                .update_and_save_single_ticker(&mut tc, &mut ticker)
                .await
            {
                error!("Ticker {}: {}", symbol, e);
                continue;
            } 
            sleep(delay).await;
            // break;
        }
        Ok(())
    }

    pub async fn handle_ticker_embeddings_eod(&self) -> Result<()> {

        let tickers = self.storage_service.get_tickers().await?;
        let symbols = tickers.iter().map(|t| t.symbol.clone());

        let length = tickers.len();
        for (i, symbol) in symbols.enumerate() {
            let mut ticker = self.storage_service.get_ticker(&symbol).await?;
            if i%20 == 0 {
                info!("Updating Ticker: {} {}/{}", ticker.symbol, i+1, length);
            }
            if let Err(e) = self
                .update_single_ticker_embedding(&mut ticker)
                .await
             {
                error!("Ticker overview embedding {}: {}", symbol, e);
                continue;
            } // break;
        }

        Ok(())
    }


    pub async fn handle_ticker_predictions_eod(&self) -> Result<()> {

        let tickers = self.storage_service.get_tickers().await?;
        let symbols = tickers.iter().map(|t| t.symbol.clone());

        let length = tickers.len();
        for (i, symbol) in symbols.enumerate() {
            let mut ticker = self.storage_service.get_ticker(&symbol).await?;
            if i%20 == 0 {
                info!("Updating Ticker: {} {}/{}", ticker.symbol, i+1, length);
            }
            if let Err(e) = self
                .update_single_ticker_prediction_signals(&mut ticker)
                .await
            {
                error!("Ticker overview embedding {}: {}", symbol, e);
                continue;
            } // break;
        }

        Ok(())
    }

}
