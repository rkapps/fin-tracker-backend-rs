use std::path::PathBuf;

use anyhow::Result;
use bin_shared::{
    logger::set_logger,
    services::{get_load_service, get_ml_service, get_pipeline_service},
};
use clap::{Parser, Subcommand};
use fin_tracker_admin::{
    seed::{load_ticker_seeds_from_file, load_ticker_seeds_from_gcs},
    ticker::{check_ticker_sentiment, check_update_ticker},
};
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "admin")]
struct Cli {
    #[command(subcommand)]
    command: AdminCommands,
}

#[derive(Subcommand)]
enum AdminCommands {
    LoadTickers {
        #[arg(short, long)]
        file: PathBuf,
    },
    TickersEod {
        #[arg(short, long)]
        symbols: Option<String>,
    },
    CheckUpdateTicker {
        #[arg(short, long)]
        symbol: String,
    },
    CheckTickerSentiment {
        #[arg(short, long)]
        symbol: String,
    },
    BuildTickerPredictionModels {
        #[arg(short, long)]
        symbols: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    set_logger();
    let cli = Cli::parse();

    match cli.command {
        AdminCommands::TickersEod { symbols } => {
            let pipeline_service = get_pipeline_service().await?;

            info!("Tickers EOD PipeLine started...");
            let symbols_str = symbols.as_deref().unwrap_or("");

            match pipeline_service.update_tickers_eod(&symbols_str).await {
                Ok(_) => info!("Tickers EOD update completed successfully."),
                Err(e) => error!("Tickers EOD update failed: {:?}", e),
            }

            match pipeline_service
                .update_ticker_eod_prediction_signals(&symbols_str)
                .await
            {
                Ok(_) => info!("Tickers EOD prediction signals completed successfully."),
                Err(e) => error!("Tickers EOD prediction signals failed: {:?}", e),
            }
            info!("Tickers EOD PipeLine done.");
        }
        AdminCommands::LoadTickers { file } => {
            let load_service = get_load_service().await?;
            let file_path = if file.to_str().unwrap_or("").starts_with("gs://") {
                load_ticker_seeds_from_gcs(file.to_str().unwrap()).await?
            } else {
                file
            };
            let ticker_seeds = load_ticker_seeds_from_file(file_path)?;

            info!("Load Tickers PipeLine started...");

            match load_service.load_tickers(&ticker_seeds).await {
                Ok(_) => info!("Background Tickers EOD Update completed successfully."),
                Err(e) => error!("Background Tickers EOD Update failed: {:?}", e),
            }

            info!("Load Tickers PipeLine done.");
        }
        AdminCommands::CheckUpdateTicker { symbol } => {
            check_update_ticker(&symbol).await?;
        }

        AdminCommands::CheckTickerSentiment { symbol } => {
            info!("Checking Ticker sentiment {}...", symbol);
            check_ticker_sentiment(&symbol).await?;
        }

        AdminCommands::BuildTickerPredictionModels { symbols } => {
            let symbols_str = symbols.as_deref().unwrap_or("");
            info!("Building Ticker Prediction Models {}...", symbols_str);
            let ml_service = get_ml_service().await?;
            let _ = ml_service.build_ticker_prediction_models(symbols_str).await;
            info!("Building Ticker Prediction Models done.");
        }
    }

    Ok(())
}
