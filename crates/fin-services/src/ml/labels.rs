use anyhow::Result;
use fin_domain::ticker::{FeatureSnapshot, IndicatorSnapshot, TickerIndicator};
use rust_decimal::prelude::ToPrimitive;
use tracing::debug;


/// Single-ticker version — used by train_for_key for both sector and ticker models.
/// No cross-ticker contamination possible since data is already isolated.
pub fn build_labels_for_ticker(
    indicators: &[TickerIndicator],
    n: usize,
) -> Result<Vec<(f64, Vec<f64>)>> {
    let length = indicators.len();
    let mut raw_labels = Vec::new();

    // Start at 1 — skip first record, prev always exists
    for index in 1..length {
        if index + n >= length {
            continue;
        }

        let isnapshot = IndicatorSnapshot::from(&indicators[index]);
        let prev_snapshot = Some(IndicatorSnapshot::from(&indicators[index - 1]));

        let fsnapshot =
            FeatureSnapshot::from_indicator_with_prev(&isnapshot, prev_snapshot.as_ref())?;

        let future = IndicatorSnapshot::from(&indicators[index + n]);

        let Some(current_price) = isnapshot.price else {
            continue;
        };
        let Some(future_price) = future.price else {
            continue;
        };
        if current_price.is_zero() {
            continue;
        }

        let return_pct = ((future_price - current_price) / current_price)
            .to_f64()
            .unwrap_or(0.0)
            * 100.0;

        raw_labels.push((return_pct, fsnapshot.values()));
        debug!(
            "  Indicators {} price: {:.2} --RSI: {:?}:{:?}:{:?} - divergence: {:?} --SMA: {:?}:{:?}:{:?} - : {:?} {:?} {:?}",
            &indicators[index].date, 
            current_price,
            isnapshot.rsi_10, 
            isnapshot.rsi_14, 
            isnapshot.rsi_26, 
            fsnapshot.values().get(1).unwrap(),
            isnapshot.sma_20,
            isnapshot.sma_100,
            isnapshot.sma_200,
            fsnapshot.values().get(2).unwrap(),
            fsnapshot.values().get(3).unwrap(),
            fsnapshot.values().get(4).unwrap()
        );
        // info!("  Features: {:?}", fsnapshot.values())
    }
 
    // Clip outliers — splits, delistings, bad EOD data
    let clipped_labels: Vec<(f64, Vec<f64>)> = raw_labels
        .into_iter()
        .filter(|(label, _)| *label >= -50.0 && *label <= 50.0)
        .collect();

    Ok(clipped_labels)
}

/// Returns (means, stds) from training data — store these and use at inference.
/// Call this on your training set, save the output, load it when predicting live.
pub fn compute_normalization_params(data: &[(f64, Vec<f64>)]) -> (Vec<f64>, Vec<f64>) {
    let n_samples = data.len();
    let n_features = data[0].1.len();

    let mut means = vec![0.0f64; n_features];
    for (_, features) in data {
        for (j, &val) in features.iter().enumerate() {
            means[j] += val;
        }
    }
    means.iter_mut().for_each(|m| *m /= n_samples as f64);

    let mut stds = vec![0.0f64; n_features];
    for (_, features) in data {
        for (j, &val) in features.iter().enumerate() {
            stds[j] += (val - means[j]).powi(2);
        }
    }
    stds.iter_mut()
        .for_each(|s| *s = (*s / n_samples as f64).sqrt().max(1e-8));

    (means, stds)
}

/// Apply stored normalization params to a single live FeatureSnapshot.
/// Use the means/stds from compute_normalization_params on your training set.
pub fn normalize_single(features: Vec<f64>, means: &[f64], stds: &[f64]) -> Vec<f64> {
    features
        .iter()
        .enumerate()
        .map(|(j, &val)| (val - means[j]) / stds[j])
        .collect()
}
