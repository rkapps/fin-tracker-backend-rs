use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use chrono::{DateTime, Months, Utc};
use fin_core::ml::{common::models::RandomForestModelCache, train::train_ticker_models};
use fin_domain::tickers::TickerIndicator;
use fin_storage::service::StorageService;
use tokio::sync::RwLock;
use tracing::{debug, info, trace, warn};


// const PERIODS: [i32; 1] = [20];
const PERIODS: [i32; 4] = [5, 10, 20, 60];
const MIN_SAMPLES: usize = 20;

#[derive(Clone)]
pub struct MlService {
    pub storage_service: Arc<dyn StorageService>,
    rf_models: RandomForestModelCache, // ✅ Clean!
}
impl std::fmt::Debug for MlService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MlService").finish()
    }
}

impl MlService {
    pub fn new(storage_service: Arc<dyn StorageService>) -> Self {
        let rf_models = Arc::new(RwLock::new(HashMap::new()));
        Self {
            storage_service,
            rf_models,
        }
    }

    pub async fn build_ticker_prediction_models(&self, symbols: &str) -> Result<()> {
        let from_date = Utc::now().checked_sub_months(Months::new(36)).unwrap();
        info!("Symbols: {}", symbols);
        let data = self.build_tickers_indicators(from_date, symbols).await?;
        info!("Data: {}", data.len());

        let result = train_ticker_models(&data, &PERIODS)?;
        // let result = self.build_and_train_by_ticker(from_date).await?;
        let _ = self.storage_service.save_ticker_alphas(&result.0).await;
        for value in result.1 {
            let mut lock = self.rf_models.write().await;
            lock.insert(value.0, Some(value.1));
        }
        Ok(())
    }

    pub async fn build_tickers_indicators(
        &self,
        from_date: DateTime<Utc>,
        symbols: &str,
    ) -> Result<Vec<(String, Vec<TickerIndicator>)>> {
        // param.limit = Some(10);
        let list: Vec<String> = symbols.split(',').map(|s| s.to_string()).collect();
        let tickers = self.storage_service.get_tickers_by_symbols(list).await?;

        let length = tickers.len();
        debug!("Tickers: {}", tickers.len());

        let mut ticker_data: Vec<(String, Vec<TickerIndicator>)> = Vec::new();
        for (i, ticker) in tickers.iter().enumerate() {
            if i % 20 == 0 {
                info!("Fetching Ticker: {} {}/{}", ticker.symbol, i + 1, length);
            }
            let indicators = self
                .storage_service
                .get_ticker_indicators_by_symbol(&ticker.symbol, from_date)
                .await?;

            if indicators.len() < MIN_SAMPLES {
                warn!(
                    "Ticker {} insufficient samples: {} rows",
                    ticker.symbol,
                    indicators.len()
                );
                continue;
            }

            trace!("key: {} indicators: {}", ticker.symbol, indicators.len());
            ticker_data.push((ticker.symbol.clone(), indicators));
        }

        Ok(ticker_data)
    }

    
}
