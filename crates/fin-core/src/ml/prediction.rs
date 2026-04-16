use std::{
    collections::HashMap,
    panic,
    sync::{Arc, RwLock},
};

use anyhow::Result;

use fin_domain::tickers::{FeatureSnapshot, IndicatorSnapshot, ModelAlgorithm, TickerAlpha, TickerIndicator};
use linfa_ensemble::EnsembleLearner;
use linfa_trees::DecisionTree;
use tracing::{debug, info, warn};

use crate::ml::{
    lr::linfa::run_lr_predictions, mlp::train::predict_mlp, rf::train::run_rf_predictions,
};

/// Prefer ticker model, fall back to sector model
pub async fn run_ticker_predictions(
    rf_models: Arc<RwLock<HashMap<String, Option<EnsembleLearner<DecisionTree<f64, usize>>>>>>,
    indicator: &TickerIndicator,
    prev_indicator: Option<&TickerIndicator>,
    ticker_alphas: &Vec<TickerAlpha>,
    sector_alphas: Vec<TickerAlpha>,
) -> Result<(
    HashMap<String, f64>,
    HashMap<String, f64>,
    HashMap<String, f64>,
)> {
    let sas = if !ticker_alphas.is_empty() {
        info!("Using ticker model for {}", indicator.symbol);
        ticker_alphas
    } else {
        info!("Falling back to sector model for {}", indicator.symbol);
        &sector_alphas
    };

    run_predictions(rf_models, indicator, prev_indicator, sas.to_vec()).await
}

pub async fn run_predictions(
    rf_models: Arc<RwLock<HashMap<String, Option<EnsembleLearner<DecisionTree<f64, usize>>>>>>,
    indicator: &TickerIndicator,
    prev_indicator: Option<&TickerIndicator>,
    sas: Vec<TickerAlpha>,
) -> Result<(
    HashMap<String, f64>,
    HashMap<String, f64>,
    HashMap<String, f64>,
)> {
    let isnapshot = IndicatorSnapshot::from(indicator);
    info!("i am here-21: {:?}", prev_indicator);

    let prev_snapshot = prev_indicator.map(|p| IndicatorSnapshot::from(p));
    info!("i am here-2");

    let fsnapshot = match panic::catch_unwind(|| {
        FeatureSnapshot::from_indicator_with_prev(&isnapshot, prev_snapshot.as_ref())
    }) {
        Ok(Ok(fs)) => fs,
        Ok(Err(e)) => {
            warn!("FeatureSnapshot error for {}: {}", indicator.symbol, e);
            return Ok((HashMap::new(), HashMap::new(), HashMap::new()));
        }
        Err(_) => {
            warn!(
                "FeatureSnapshot panicked for {} — likely division by zero in indicators",
                indicator.symbol
            );
            return Ok((HashMap::new(), HashMap::new(), HashMap::new()));
        }
    };

    let mut lf_returns = HashMap::new();
    let mut rf_returns = HashMap::new();
    let mut mlp_returns = HashMap::new();

    for sa in sas {
        if sa.means.len() != fsnapshot.values().len() {
            warn!(
                "Period {} feature mismatch: model={} snapshot={}, skipping",
                sa.n,
                sa.means.len(),
                fsnapshot.values().len()
            );
            continue;
        }

        let normalized = normalize_single(fsnapshot.values(), &sa.means, &sa.stds);
        info!("i am here");
        match sa.model_algorithm {
            ModelAlgorithm::LinearRegression => {
                let predicted_return = run_lr_predictions(&sa, &normalized);
                lf_returns.insert(sa.n.to_string(), predicted_return);
                debug!(
                    "Period {} algorithm: {:?} predicted: {:.2}%",
                    sa.id, sa.model_algorithm, predicted_return
                );
            }
            ModelAlgorithm::RandomForest => {
                let key = format!("{}:{}", sa.key, sa.n);
                let lock = rf_models.read().unwrap();
                let Some(Some(model)) = lock.get(&key) else {
                    // warn!("RF model not found for {}", sa.id);
                    continue;
                };
                match run_rf_predictions(&sa, model, normalized) {
                    Ok(c) => {
                        rf_returns.insert(sa.n.to_string(), c);
                        debug!(
                            "Period {} algorithm: {:?} predicted: {:.2}%",
                            sa.id, sa.model_algorithm, c
                        );
                    }
                    Err(e) => {
                        return Err(anyhow::anyhow!("RF Prediction error: {}", e));
                    }
                };
            }
            ModelAlgorithm::MLP => match predict_mlp(&sa, normalized) {
                Ok(c) => {
                    mlp_returns.insert(sa.n.to_string(), c);

                    info!("predictions for period:{} : {:?}", sa.n, c);
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("MLP Prediction error: {}", e));
                }
            },
        };
    }

    Ok((lf_returns, rf_returns, mlp_returns))
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
