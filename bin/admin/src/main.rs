use std::path::PathBuf;

use anyhow::Result;
use bin_shared::{logger::set_logger, services::get_load_service};
use clap::{Parser, Subcommand};
use fin_tracker_admin::{seed::load_ticker_seeds_from_file, ticker::{check_ticker_sentiment, check_update_ticker}};
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
    CheckUpdateTicker {
        #[arg(short, long)]
        symbol: String,
    }, // LoadExchanges,
    CheckTickerSentiment {
        #[arg(short, long)]
        symbol: String,
    }, // LoadExchanges,
       // FixData,
}

#[tokio::main]
async fn main() -> Result<()> {
    set_logger();
    let cli = Cli::parse();

    let load_service = get_load_service().await?;

    match cli.command {
        AdminCommands::LoadTickers { file } => {
            let ticker_seeds = load_ticker_seeds_from_file(file)?;

            info!("Load Tickers PipeLine started...");

            match load_service.load_tickers(&ticker_seeds).await {
                Ok(_) => info!("Background Tickers EOD Update completed successfully."),
                Err(e) => error!("Background Tickers EOD Update failed: {:?}", e),
            }

            // match load_service.load_ticker_embeddings(&ticker_seeds).await {
            //     Ok(_) => info!("Background Tickers EOD Update completed successfully."),
            //     Err(e) => error!("Background Tickers EOD Update failed: {:?}", e),
            // }
            info!("Load Tickers PipeLine done.");
        }
        AdminCommands::CheckUpdateTicker { symbol } => {
            check_update_ticker(&symbol).await?;
        }

        AdminCommands::CheckTickerSentiment { symbol } => {
            info!("Checking Ticker sentiment {}...", symbol);
            check_ticker_sentiment(&symbol).await?;
        }
    }

    Ok(())
}
