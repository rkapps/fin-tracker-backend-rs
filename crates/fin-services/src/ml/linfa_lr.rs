use anyhow::Result;
use fin_domain::ticker::TickerAlpha;
use linfa::prelude::*;
use linfa_linear::LinearRegression;
use ndarray::{Array1, Array2};
use tracing::debug;

use crate::ml::{
    metrics::log_metrics_from_vecs,
    models::{LrResult, ModelMetrics},
};

/// Build a scaled linfa Dataset using pre-computed normalization params.
/// means and stds must come from compute_normalization_params() in labels.rs
/// called on the TRAINING set only — never recompute on test or live data.
pub fn build_linfa_dataset(
    labeled_data: &[(f64, Vec<f64>)],
    means: &[f64],
    stds: &[f64],
) -> Result<Dataset<f64, f64, ndarray::Dim<[usize; 1]>>> {
    let dataset = apply_scaling(labeled_data, means, stds)?;

    debug!(
        "Training dataset — samples: {}  features: {}",
        dataset.nsamples(),
        dataset.nfeatures()
    );

    Ok(dataset)
}

/// Scale test or live data using the training set's normalization params.
/// Signature identical to build_linfa_dataset for consistency.
pub fn scale_dataset(
    labeled_data: &[(f64, Vec<f64>)],
    means: &[f64],
    stds: &[f64],
) -> Result<Dataset<f64, f64, ndarray::Dim<[usize; 1]>>> {
    apply_scaling(labeled_data, means, stds)
}

/// Shared scaling logic — applies z-score normalization using provided params.
fn apply_scaling(
    labeled_data: &[(f64, Vec<f64>)],
    means: &[f64],
    stds: &[f64],
) -> Result<Dataset<f64, f64, ndarray::Dim<[usize; 1]>>> {
    let n_samples = labeled_data.len();
    let n_features = labeled_data
        .last()
        .ok_or_else(|| anyhow::anyhow!("Empty dataset"))?
        .1
        .len();

    anyhow::ensure!(
        means.len() == n_features && stds.len() == n_features,
        "Normalization param count ({}) does not match feature count ({})",
        means.len(),
        n_features
    );

    let scaled_x: Vec<f64> = labeled_data
        .iter()
        .flat_map(|(_, features)| {
            features.iter().enumerate().map(|(i, v)| {
                if stds[i] > 1e-8 {
                    (v - means[i]) / stds[i]
                } else {
                    0.0
                }
            })
        })
        .collect();

    let y: Vec<f64> = labeled_data.iter().map(|(label, _)| *label).collect();

    let x_matrix = Array2::from_shape_vec((n_samples, n_features), scaled_x)
        .map_err(|e| anyhow::anyhow!("Failed to build feature matrix: {}", e))?;
    let y_vector = Array1::from_vec(y);

    debug!("First row after scaling: {:?}", x_matrix.row(0));

    Ok(Dataset::new(x_matrix, y_vector))
}

pub fn train_models_for_linfa(
    _labeled_data: &Vec<(f64, Vec<f64>)>,
    train_data: &[(f64, Vec<f64>)],
    test_data: &[(f64, Vec<f64>)],
    means: &[f64],
    stds: &[f64],
) -> Result<LrResult> {
    let train_dataset = build_linfa_dataset(train_data, means, stds)?;
    let test_dataset = scale_dataset(test_data, &means, &stds)?;

    let model = LinearRegression::default().fit(&train_dataset)?;
    let predictions = model.predict(&test_dataset);
    let actuals = test_dataset.targets();

    let params = model.params().to_vec();
    let intercept = model.intercept();

    // In LR branch — unify to same metrics function
    let float_predictions: Vec<f64> = predictions.iter().copied().collect();
    let float_actuals: Vec<f64> = actuals.iter().copied().collect();

    debug!("Intercept: {:.4}", intercept);
    debug!("Params:    {:?}", params);
    let (directional_accuracy, bullish_precision, bearish_precision, mae, r2) =
        log_metrics_from_vecs(&float_predictions, &float_actuals)?;
    let metrics = ModelMetrics {
        bearish_precision,
        bullish_precision,
        directional_accuracy,
        mae,
        r2,
    };

    Ok(LrResult {
        intercept,
        params,
        metrics,
    })
}


pub fn run_lr_predictions(sa: &TickerAlpha, normalized: &[f64]) -> f64{
    let raw_return: f64 = sa.intercept
        + normalized
            .iter()
            .zip(sa.params.iter())
            .map(|(scaled, p)| scaled * p)
            .sum::<f64>();

    // Round to 2 decimal places
    let predicted_return = (raw_return * 100.0).round() / 100.0;
    predicted_return
}
