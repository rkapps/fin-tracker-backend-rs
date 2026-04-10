use anyhow::Result;
use fin_domain::ticker::TickerIndicator;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tracing::{debug, info, warn};

use crate::ml::common::labels::build_labels;


pub async fn build_tickers_models(
    data: &[(String, Vec<TickerIndicator>)],
    periods: &[i32],
) -> Result<()> {
    debug!("build_tickers_models: {}", data.len());
    data.par_iter().for_each(|(key, indicators)| {
        info!("Key: {} indicators: {}", key, indicators.len());
        for period in periods {
            // build_labels(indicators, *period as usize).await?;
            let ticker_labels = match build_labels(indicators, *period as usize) {
                Ok(labels) => labels,
                Err(e) => {
                    warn!("Label build failed {} period {}: {}", key, period, e);
                    continue;
                }
            };

            debug!("Labels: {:?}", ticker_labels);
        }
    });

    Ok(())
}
