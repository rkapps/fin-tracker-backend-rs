use anyhow::Result;
use bin_shared::services::get_pipeline_service;
use fin_core::tickers::update::update_ticker;
use rust_decimal_macros::dec;
use tracing::{debug, error};

pub async fn check_update_ticker(symbol: &str) -> Result<()> {
    let pipeline_service = get_pipeline_service().await?;

    // Get ticker data
    let mut tc = match pipeline_service
        .storage_service
        .get_ticker_control(symbol)
        .await
    {
        Ok(tc) => tc,
        Err(e) => {
            error!("Failed to get ticker control for {}: {}", symbol, e);
            return Err(e);
        }
    };

    let mut ticker = match pipeline_service
        .storage_service
        .get_ticker_by_symbol(symbol)
        .await
    {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to get ticker {}: {}", symbol, e);
            return Err(e);
        }
    };

    // Update and save
    if let Err(e) = update_ticker(
        pipeline_service.storage_service,
        pipeline_service.provider_service,
        pipeline_service.embedding_client,
        &mut tc,
        &mut ticker,
        false
    )
    .await
    {
        error!("Ticker {}: {}", symbol, e);
        return Err(e);
    }

    Ok(())
}

pub async fn check_ticker_sentiment(symbol: &str) -> Result<()> {
    let pipeline_service = get_pipeline_service().await?;
    let score = dec!(0.9);

    let sentiments = pipeline_service
        .storage_service
        .get_ticker_sentiments_with_score(symbol, &score)
        .await?;
    debug!("Sentiments: {}", sentiments.len());

    Ok(())
}
