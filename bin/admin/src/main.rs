use std::path::PathBuf;

use anyhow::Result;
use bin_shared::{logger::set_logger, services::get_load_service};
use clap::{Parser, Subcommand};
use fin_tracker_admin::load_from_file::load_tickers_from_file;
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
    // LoadExchanges,
    // FixData,
}

#[tokio::main]
async fn main() -> Result<()> {
    set_logger();
    let cli = Cli::parse();

    let load_service = get_load_service().await?;

    match cli.command {
        AdminCommands::LoadTickers { file } => {
            let ticker_seeds = load_tickers_from_file(file)?;

            info!("Load Tickers PipeLine started...");

            match load_service.load_tickers(ticker_seeds).await {
                Ok(_) => info!("Background Tickers EOD Update completed successfully."),
                Err(e) => error!("Background Tickers EOD Update failed: {:?}", e),
            }

            match load_service.load_ticker_embeddings().await {
                Ok(_) => info!("Background Tickers EOD Update completed successfully."),
                Err(e) => error!("Background Tickers EOD Update failed: {:?}", e),
            }
            info!("Load Tickers PipeLine done.");
        }
    }

    Ok(())
}
