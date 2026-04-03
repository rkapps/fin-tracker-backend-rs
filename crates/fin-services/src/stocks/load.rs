use std::time::Duration;

use crate::stocks::StocksService;
use anyhow::Result;
use fin_domain::ticker::{Ticker, TickerControl, TickerSeed};
use tokio::time::sleep;
use tracing::{error, info};

impl StocksService {
    pub async fn load_tickers(&self, ticker_seeds: Vec<TickerSeed>) -> Result<()> {
        info!("Loading tickers: {}", ticker_seeds.len());
        let delay = Duration::from_millis(500); // Sleep for 0 milliseconds

        let mut count = 0;
        let length = ticker_seeds.len();
        for (i, seed) in ticker_seeds.iter().enumerate() {
            if i % 20 == 0 {
                info!("Loading Ticker: {} {}/{}", seed.symbol, i + 1, length);
            }

            let (mut tc, mut ticker) = {
                let tc = self
                    .storage_service
                    .get_ticker_control(&seed.symbol.clone())
                    .await
                    .unwrap_or_else(|_| TickerControl::new(seed.clone()));

                let ticker = Ticker::new(seed.clone());
                (tc, ticker)
            };

            if let Err(e) = self
                .update_and_save_single_ticker(&mut tc, &mut ticker)
                .await
            {
                error!("Ticker {}: {}", seed.symbol, e);
                continue;
            }

            sleep(delay).await;
            count += 1;
            // break;
        }

        info!("Loaded {} tickers.", count);
        Ok(())
    }
}
