use std::path::PathBuf;

use anyhow::Result;
use bin_shared::{
    logger::set_logger,
    services::{get_load_service, get_ml_service, get_pipeline_service, get_storage_service},
};
use chrono::Utc;
use clap::{Parser, Subcommand};
use fin_core::tickers::update::update_ticker_overview_embedding;
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
    BuildTickerPredictionModels {
        #[arg(short, long)]
        symbols: Option<String>,
    },
    LoadTickers {
        #[arg(short, long)]
        file: PathBuf,
    },
    CheckUpdateTicker {
        #[arg(short, long)]
        symbol: String,
    },
    CheckTickerSentiment {
        #[arg(short, long)]
        symbol: String,
    },
    PruneIndicators, // keep last 5 years
    PruneSentiments, // keep last 30 days
    PruneEmbeddings, // keep last 30 days
    TickersEod {
        #[arg(short, long)]
        symbols: Option<String>,
    },
    UpdateTickerOverviewEmbeddings {
        #[arg(short, long)]
        symbols: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    set_logger();
    let cli = Cli::parse();

    match cli.command {
        AdminCommands::BuildTickerPredictionModels { symbols } => {
            let symbols_str = symbols.as_deref().unwrap_or("");
            info!("Building Ticker Prediction Models {}...", symbols_str);
            let ml_service = get_ml_service().await?;
            let _ = ml_service.build_ticker_prediction_models(symbols_str).await;
            info!("Building Ticker Prediction Models done.");
        }
        AdminCommands::CheckUpdateTicker { symbol } => {
            check_update_ticker(&symbol).await?;
        }

        AdminCommands::CheckTickerSentiment { symbol } => {
            info!("Checking Ticker sentiment {}...", symbol);
            check_ticker_sentiment(&symbol).await?;
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

        AdminCommands::PruneEmbeddings => {
            let storage_service = get_storage_service().await?;
            info!("Pruning embeddings older than 30 days...");
            let cutoff = Utc::now() - chrono::Duration::days(30);

            match storage_service.delete_ticker_embeddings_before(cutoff).await {
                Ok(_) => info!("Prune embeddings complete"),
                Err(e) => error!("Prune embeddings failed: {:?}", e),
            }
        }

        AdminCommands::PruneIndicators => {
            let storage_service = get_storage_service().await?;
            info!("Pruning indicators older than 5 years...");
            let cutoff = Utc::now() - chrono::Duration::days(365 * 5);

            match storage_service.delete_ticker_indicators_before(cutoff).await {
                Ok(_) => info!("Prune indicators complete"),
                Err(e) => error!("Prune indicators failed: {:?}", e),
            }
        }

        AdminCommands::PruneSentiments => {
            let storage_service = get_storage_service().await?;
            info!("Pruning sentiments older than 30 days...");
            let cutoff = Utc::now() - chrono::Duration::days(30);

            match storage_service.delete_ticker_sentiments_before(cutoff).await {
                Ok(_) => info!("Prune sentiments complete"),
                Err(e) => error!("Prune sentiments failed: {:?}", e),
            }
        }

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

        AdminCommands::UpdateTickerOverviewEmbeddings { symbols } => {
            let symbols_str = symbols.as_deref().unwrap_or("");

            let pipeline_service = get_pipeline_service().await?;
            let all_tickers = if symbols.is_some() {
                let list: Vec<String> = symbols_str.split(',').map(|s| s.to_string()).collect();
                pipeline_service
                    .storage_service
                    .get_tickers_by_symbols(list)
                    .await?
            } else {
                pipeline_service
                    .storage_service
                    .get_tickers_by_marketcap()
                    .await?
            };

            // run ticker overview embeddings
            for mut ticker in all_tickers {
                update_ticker_overview_embedding(
                    pipeline_service.storage_service.clone(),
                    pipeline_service.embedding_client.clone(),
                    &mut ticker,
                )
                .await?;
            }
        }
    }

    Ok(())
}
