use anyhow::Result;
use fin_domain::{
    dto::{
        screen_param::TickerScreenParam, ticker_chart_entity::TickerChartEntity,
        ticker_entity::TickerEntity,
    },
    tickers::TickerIndicator,
};
use fin_storage::service::StorageService;
use rust_decimal::{Decimal, prelude::ToPrimitive};
use std::{collections::HashMap, sync::Arc};
use tracing::{debug};

#[derive(Debug)]
pub struct TickersService {
    pub storage_service: Arc<dyn StorageService>,
}

impl TickersService {
    pub fn new(storage_service: Arc<dyn StorageService>) -> TickersService {
        TickersService { storage_service }
    }

    pub async fn get_tickers_by_symbols(&self, symbols: Vec<String>) -> Result<Vec<TickerEntity>> {
        let tickers = self
            .storage_service
            .get_tickers_by_symbols(symbols)
            .await
            .map_err(|e| anyhow::anyhow!(format!("Get Ticker error: {}", e)))?;

        let tentities = tickers
            .iter()
            .map(|t| TickerEntity::from(t.clone()))
            .collect();
        Ok(tentities)
    }

    pub async fn get_ticker_groups(&self) -> Result<HashMap<String, Vec<String>>> {
        let groups = self
            .storage_service
            .get_ticker_groups()
            .await
            .map_err(|e| anyhow::anyhow!(format!("Get Ticker Groups error: {}", e)))?;
        Ok(groups)
    }

    pub async fn get_tickers_by_function(&self, function: &str) -> Result<Vec<TickerEntity>> {
        let tentities: Vec<TickerEntity> = match function {
            "etfs" => {
                let symbols = vec![
                    "DIA".into(),
                    "SPY".into(),
                    "IWM".into(),
                    "GLD".into(),
                    "GBTC".into(),
                    "ETHE".into(),
                    "QQQ".into(),
                    "VIX".into(),
                ];
                self.get_tickers_by_symbols(symbols).await.unwrap()
            }
            "spiders" => {
                let symbols = vec![
                    "XLY".into(),
                    "XLP".into(),
                    "XLE".into(),
                    "XLF".into(),
                    "XLK".into(),
                    "XLU".into(),
                    "XHB".into(),
                ];
                self.get_tickers_by_symbols(symbols).await.unwrap()
            }
            "international" => {
                let symbols = vec![
                    "VWO".into(),
                    "VGK".into(),
                    "VXUS".into(),
                    "VEU".into(),
                    "VSGX".into(),
                    "VWOB".into(),
                    "VIGI".into(),
                    "EWZ".into(),
                    "EWJ".into(),
                ];
                self.get_tickers_by_symbols(symbols).await.unwrap()
            }
            _ => {
                let tickers = self
                    .storage_service
                    .get_tickers_by_movers(function)
                    .await
                    .unwrap();
                tickers
                    .iter()
                    .map(|t| TickerEntity::from(t.clone()))
                    .collect()
            }
        };

        Ok(tentities)
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
                        .unwrap_or( &Decimal::ZERO)
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

    pub async fn search_tickers(&self, param: TickerScreenParam) -> Result<Vec<TickerEntity>> {
        let tickers = self
            .storage_service
            .search_tickers(param)
            .await
            .map_err(|e| anyhow::anyhow!(format!("Get Ticker error: {}", e)))?;

        debug!("Tickers: {}", tickers.len());
        let tentities = tickers
            .iter()
            .map(|t| TickerEntity::from(t.clone()))
            .collect();
        Ok(tentities)
    }
}
