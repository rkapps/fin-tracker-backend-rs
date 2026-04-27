use anyhow::Result;
use bin_shared::{
    logger::set_logger,
    services::{get_ml_service, get_pipeline_service},
};
use clap::{Parser, Subcommand};
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "pipeline")]
struct Cli {
    #[command(subcommand)]
    command: PipelineCommands,
}

#[derive(Subcommand)]
enum PipelineCommands {
    TickersEod, 
    UpdateTickersNews, 
    RealtimeStocksEtfs,
    RealtimeCryptos,
    BuildTickerPredictionModels,
}

#[tokio::main]
async fn main() -> Result<()> {
    set_logger();

    let cli = Cli::parse();
    let pipeline_service = get_pipeline_service().await?;

    match cli.command {
        PipelineCommands::TickersEod => {
            info!("Tickers EOD PipeLine started...");
            match pipeline_service.update_tickers_eod("", true).await {
                Ok(_) => info!("Tickers EOD update completed successfully."),
                Err(e) => error!("Tickers EOD update failed: {:?}", e),
            }

            match pipeline_service
                .update_ticker_eod_prediction_signals("")
                .await
            {
                Ok(_) => info!("Tickers EOD prediction signals completed successfully."),
                Err(e) => error!("Tickers EOD prediction signals failed: {:?}", e),
            }
            info!("Tickers EOD PipeLine done.");
        }
        PipelineCommands::RealtimeStocksEtfs => {
            info!("Tickers Stocks and Etfs Realtime started...");
            match pipeline_service.update_realtime_stocks_etfs("", true).await {
                Ok(_) => info!("Tickers Stocks and Etfs Realtime completed successfully."),
                Err(e) => error!("Tickers Stocks and Etfs Realtime failed: {:?}", e),
            }
        }
        PipelineCommands::RealtimeCryptos => {
            info!("Tickers Crypto Realtime started...");
            match pipeline_service.update_realtime_cryptos("", true).await {
                Ok(_) => info!("Tickers Crypto Realtime completed successfully."),
                Err(e) => error!("Tickers Crypto Realtime failed: {:?}", e),
            }
        }
        PipelineCommands::BuildTickerPredictionModels => {
            info!("Tickers Training Model started...");
            let ml_service = get_ml_service().await?;
            match ml_service.build_ticker_prediction_models("").await {
                Ok(_) => info!("Tickers Training Model completed successfully."),
                Err(e) => error!("Tickers Training Model failed: {:?}", e),
            }
        }
        PipelineCommands::UpdateTickersNews => {
            info!("Tickers News PipeLine started...");
            match pipeline_service.update_tickers_news().await {
                Ok(_) => info!("Tickers News update completed successfully."),
                Err(e) => error!("Tickers News update failed: {:?}", e),
            }
        }


    }

    Ok(())
}
