use anyhow::Result;
use fin_domain::tickers::TickerAlpha;
use linfa::prelude::*;
use linfa_ensemble::{EnsembleLearner, EnsembleLearnerParams};
use linfa_trees::{DecisionTree, SplitQuality};
use ndarray::{Array1, Array2, Ix1};

use crate::ml::common::{
    metrics::log_metrics_from_vecs,
    models::{ModelMetrics, RfResult},
};

pub fn build_rf_dataset(
    labeled_data: &[(f64, Vec<f64>)],
    means: &[f64],
    stds: &[f64],
) -> Result<Dataset<f64, usize, Ix1>> {
    let n_samples = labeled_data.len();
    let n_features = labeled_data
        .last()
        .ok_or_else(|| anyhow::anyhow!("Empty dataset"))?
        .1
        .len();

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

    // Convert continuous return % → class 0/1/2
    let y: Vec<usize> = labeled_data
        .iter()
        .map(|(label, _)| to_class(*label))
        .collect();

    let x_matrix = Array2::from_shape_vec((n_samples, n_features), scaled_x)?;
    let y_vector = Array1::from_vec(y);

    Ok(Dataset::new(x_matrix, y_vector))
}

// Convert continuous return % → class index
pub fn to_class(return_pct: f64) -> usize {
    if return_pct > 2.0 {
        2
    }
    // UP
    else if return_pct < -2.0 {
        0
    }
    // DOWN
    else {
        1
    } // NEUTRAL
}

pub fn train_models_for_randomforest(
    labeled_data: &[(f64, Vec<f64>)],
    train_data: &[(f64, Vec<f64>)],
    test_data: &[(f64, Vec<f64>)],
    means: &[f64],
    stds: &[f64],
) -> Result<RfResult> {
    // Cap the RF samples to 20K, find the lastest 20K
    let max_rf_samples = 20_000;
    let train_data = if train_data.len() > max_rf_samples {
        &train_data[train_data.len() - max_rf_samples..] // keep most recent
    } else {
        train_data
    };

    let train_dataset = build_rf_dataset(train_data, means, stds)?;
    let test_dataset = build_rf_dataset(test_data, means, stds)?;
    let model = EnsembleLearnerParams::new(
        DecisionTree::params()
            .split_quality(SplitQuality::Gini)
            .max_depth(Some(4))
            .min_weight_split(50.0),
    )
    .ensemble_size(20)
    .bootstrap_proportion(0.8)
    .fit(&train_dataset)?;

    // Compute mean return per class from labeled data
    let mean_up = labeled_data
        .iter()
        .filter(|(l, _)| *l > 2.0)
        .map(|(l, _)| l)
        .sum::<f64>()
        / labeled_data.iter().filter(|(l, _)| *l > 2.0).count().max(1) as f64;

    let mean_neutral = labeled_data
        .iter()
        .filter(|(l, _)| *l >= -2.0 && *l <= 2.0)
        .map(|(l, _)| l)
        .sum::<f64>()
        / labeled_data
            .iter()
            .filter(|(l, _)| *l >= -2.0 && *l <= 2.0)
            .count()
            .max(1) as f64;

    let mean_down = labeled_data
        .iter()
        .filter(|(l, _)| *l < -2.0)
        .map(|(l, _)| l)
        .sum::<f64>()
        / labeled_data
            .iter()
            .filter(|(l, _)| *l < -2.0)
            .count()
            .max(1) as f64;

    let predictions = model.predict(&test_dataset);
    // Convert class predictions → f64 returns using class means
    let float_predictions: Vec<f64> = predictions
        .iter()
        .map(|class| match class {
            2 => mean_up,
            0 => mean_down,
            _ => mean_neutral,
        })
        .collect();

    // Actual float labels come from test_data, not test_dataset
    let float_actuals: Vec<f64> = test_data.iter().map(|(label, _)| *label).collect();

    let (directional_accuracy, bullish_precision, bearish_precision, mae, r2) =
        log_metrics_from_vecs(&float_predictions, &float_actuals)?;
    let metrics = ModelMetrics {
        bearish_precision,
        bullish_precision,
        directional_accuracy,
        mae,
        r2,
    };

    Ok(RfResult {
        model,
        metrics,
        mean_down,
        mean_neutral,
        mean_up,
    })
}

pub fn run_rf_predictions(
    sa: &TickerAlpha,
    model: &EnsembleLearner<DecisionTree<f64, usize>>,
    normalized: Vec<f64>,
) -> Result<f64> {
    // Build a single-row dataset from normalized features
    let x = Array2::from_shape_vec((1, normalized.len()), normalized)?;
    let dataset = Dataset::new(x, Array1::from_vec(vec![0usize]));

    let prediction = model.predict(&dataset);
    let class = prediction[0];

    // Map class → return using stored means
    let rf_return = match class {
        2 => sa.mean_up,
        0 => sa.mean_down,
        _ => sa.mean_neutral,
    };

    // Round to 2 decimal places
    let predicted_return = (rf_return * 100.0).round() / 100.0;
    Ok(predicted_return)
}
