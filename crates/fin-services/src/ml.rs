use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use chrono::{DateTime, Months, Utc};
use fin_core::ml::{
    common::models::{RandomForestModel, RandomForestModelCache},
    train::train_ticker_models,
};
use fin_domain::tickers::TickerAlpha;
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
        let result = self.build_models(from_date, symbols).await?;
        let _ = self.storage_service.save_ticker_alphas(result.0).await;
        // for value in result.1 {
        //     let mut lock = self.rf_models.write().await;
        //     lock.insert(value.0, Some(value.1));
        // }
        // let bytes = bincode::serialize(&result.1)?;
        // info!("RF models size: {} MB", bytes.len() / 1_000_000);
        Ok(())
    }

    pub async fn build_models(
        &self,
        from_date: DateTime<Utc>,
        symbols: &str,
    ) -> Result<(Vec<TickerAlpha>, HashMap<String, RandomForestModel>)> {
        let tickers = if !symbols.is_empty() {
            let list: Vec<String> = symbols.split(',').map(|s| s.to_string()).collect();
            self.storage_service.get_tickers_by_symbols(list).await?
        } else {
            self.storage_service.get_tickers_by_marketcap().await?
        };

        let length = tickers.len();
        debug!("Tickers: {}", tickers.len());
        let mut all_alphas = Vec::new();
        let mut all_rf_models = HashMap::new();

        for (i, ticker) in tickers.iter().enumerate() {
            info!("Training Ticker: {} {}/{}", ticker.symbol, i + 1, length);
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

            let result = train_ticker_models(&[(ticker.symbol.clone(), indicators)], &PERIODS)?;
            // collect results in memory - small structs only
            all_alphas.extend(result.0);
            for (key, model) in result.1 {
                all_rf_models.insert(key, model);
            }
        }

        Ok((all_alphas, all_rf_models))
    }
}
