use std::sync::Arc;

use anyhow::Result;
use fin_domain::dto::screen_param::TickerScreenParam;
use fin_services::stocks::StocksService;
use tracing::{error, info};


pub async fn fix_ticker_history(stocks_service: Arc<StocksService>) -> Result<()>{
    
    let asset_type = "CRYPTO";
    let param = TickerScreenParam::new_for_asset_type(asset_type);
    let tickers = stocks_service.storage_service.search_tickers(param).await.unwrap();

    for ticker in tickers {
        info!("Ticker: {}", ticker.symbol);

        let mut tc = match stocks_service.storage_service.get_ticker_control(&ticker.symbol).await {
            Ok(tc) => tc,
            Err(e) => {
                error!("Failed to get ticker control for {}: {}", ticker.symbol, e);
                return Err(e);
            }
        };

        match stocks_service.storage_service.delete_ticker_history(&ticker.symbol).await {
            Ok(_) => info!("Ticker History deleted"),
            Err(e) => error!("Ticker History deletion failed: {:?}", e),
        }

        match stocks_service.storage_service.delete_ticker_indicators(&ticker.symbol).await {
            Ok(_) => info!("Ticker Indicators deleted"),
            Err(e) => error!("Ticker Indicators deletion failed: {:?}", e),
        }

        tc.last_history_sync_at = None;
        tc.last_indicator_sync_at = None;

        stocks_service.storage_service.save_ticker_control(tc.clone()).await?;

    }
    Ok(())
}