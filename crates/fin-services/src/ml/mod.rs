use crate::stocks::StocksService;
use anyhow::Result;
use std::sync::Arc;

pub struct MlService {
    pub stocks_service: Arc<StocksService>,
}

impl MlService {
    pub fn new(stocks_service: Arc<StocksService>) -> Self {
        Self { stocks_service }
    }

    pub async fn train_model(self) -> Result<()> {
        let tickers = self.stocks_service.storage_service.get_tickers().await?;
        for ticker in tickers {
            let indicators = self
                .stocks_service
                .storage_service
                .get_ticker_indicators(&ticker.symbol)
                .await;
        }

        Ok(())
    }
}
