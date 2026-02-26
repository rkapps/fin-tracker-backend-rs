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
        for symbol in symbols {
            let mut tc = self.storage_service.get_ticker_control(&symbol).await?;
            let mut ticker = self.storage_service.get_ticker(&symbol).await?;
            info!("Updating Ticker: {}", ticker.symbol);
            if let Err(e) = self
                .update_and_save_single_ticker(&mut tc, &mut ticker)
                .await
            {
                error!("Ticker {}: {}", symbol, e);
                continue;
            } // break;
            sleep(delay).await;
        }
        Ok(())
    }

    pub async fn handle_ticker_embeddings_eod(&self) -> Result<()> {

        let tickers = self.storage_service.get_tickers().await?;
        let symbols = tickers.iter().map(|t| t.symbol.clone());

        for symbol in symbols {
            info!("Updating Ticker: {}", symbol);
            let mut ticker = self.storage_service.get_ticker(&symbol).await?;
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
}
