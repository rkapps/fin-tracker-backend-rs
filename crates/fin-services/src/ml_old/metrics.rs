use anyhow::Result;

// ── Metrics ───────────────────────────────────────────────────────────
pub(crate) fn log_metrics_from_vecs(
    predictions: &[f64],
    actuals: &[f64],
) -> Result<(f64, f64, f64, f64, f64)> {
    let total = predictions.len();
    if total == 0 {
        return Err(anyhow::anyhow!("Empty predictions"));
    }

    let correct_direction = predictions
        .iter()
        .zip(actuals.iter())
        .filter(|(pred, actual)| pred.signum() == actual.signum())
        .count();
    let directional_accuracy = correct_direction as f64 / total as f64;

    let mae = predictions
        .iter()
        .zip(actuals.iter())
        .map(|(pred, actual)| (pred - actual).abs())
        .sum::<f64>()
        / total as f64;

    let bullish_total = predictions.iter().filter(|p| **p > 0.0).count();
    let correct_bullish = predictions
        .iter()
        .zip(actuals.iter())
        .filter(|(pred, actual)| **pred > 0.0 && **actual > 0.0)
        .count();
    let bullish_precision = if bullish_total > 0 {
        correct_bullish as f64 / bullish_total as f64
    } else {
        0.0
    };

    let bearish_total = predictions.iter().filter(|p| **p < 0.0).count();
    let correct_bearish = predictions
        .iter()
        .zip(actuals.iter())
        .filter(|(pred, actual)| **pred < 0.0 && **actual < 0.0)
        .count();
    let bearish_precision = if bearish_total > 0 {
        correct_bearish as f64 / bearish_total as f64
    } else {
        0.0
    };

    // R2 from vecs
    let actual_mean = actuals.iter().sum::<f64>() / total as f64;
    let ss_tot = actuals
        .iter()
        .map(|a| (a - actual_mean).powi(2))
        .sum::<f64>();
    let ss_res = predictions
        .iter()
        .zip(actuals.iter())
        .map(|(pred, actual)| (actual - pred).powi(2))
        .sum::<f64>();
    let r2 = if ss_tot > 0.0 {
        1.0 - ss_res / ss_tot
    } else {
        0.0
    };

    Ok((
        directional_accuracy,
        bullish_precision,
        bearish_precision,
        mae,
        r2,
    ))
}
