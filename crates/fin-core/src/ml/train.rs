use std::{collections::HashMap, sync::Mutex};

use anyhow::Result;
use chrono::Utc;
use fin_domain::tickers::{ModelAlgorithm, ModelType, TickerAlpha, TickerIndicator};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tracing::{debug, warn};

use crate::ml::{
    common::{labels::build_labels, models::RandomForestModel},
    lr::linfa::train_models_for_linfa,
    mlp::train::train_models_for_mlp,
    rf::train::train_models_for_randomforest,
};

pub fn train_ticker_models(
    data: &[(String, Vec<TickerIndicator>)],
    periods: &[i32],
) -> Result<(Vec<TickerAlpha>, HashMap<String, RandomForestModel>)> {
    let all_alphas = Mutex::new(Vec::new());
    let all_rf_models = Mutex::new(HashMap::new());

    debug!("Training ticker models: {}", data.len());
    data.par_iter().for_each(|(key, indicators)| {
        let mut ticker_alphas = Vec::new();
        let mut ticker_rf_models = HashMap::new();
        let mut success_count = 0;

        for period in periods {
            // build_labels(indicators, *period as usize).await?;
            let ticker_labels = match build_labels(indicators, *period as usize) {
                Ok(labels) => labels,
                Err(e) => {
                    warn!("Label build failed {} period {}: {}", key, period, e);
                    continue;
                }
            };

            debug!(
                "         Key: {} indicators: {} Period: {} Labels: {:?}",
                key,
                indicators.len(),
                period,
                ticker_labels.len()
            );

            if ticker_labels.len() < *period as usize {
                warn!(
                    "           Labels {} less than the period {}. Skipping... ",
                    ticker_labels.len(),
                    period
                );
                continue;
            }

            match train_labels_for_all_algorithms(
                key,
                "",
                ticker_labels,
                *period,
                ModelType::Ticker,
            ) {
                Ok(result) => {
                    ticker_alphas.extend(result.0);
                    ticker_rf_models.extend(result.1);
                    success_count += 1;
                }
                Err(e) => {
                    warn!("Training failed {} period {}: {}", key, period, e);
                }
            }
        }

        debug!(
            "  {} — trained {}/{} periods, {} alphas",
            key,
            success_count,
            periods.len(),
            ticker_alphas.len()
        );

        if !ticker_alphas.is_empty() {
            match all_alphas.lock() {
                Ok(mut guard) => guard.extend(ticker_alphas),
                Err(e) => warn!("Mutex poisoned for {}: {}", key, e),
            }
        }

        if !ticker_rf_models.is_empty() {
            match all_rf_models.lock() {
                Ok(mut guard) => guard.extend(ticker_rf_models),
                Err(e) => warn!("RF mutex poisoned for {}: {}", key, e),
            }
        }
    });
    // Check what was collected
    let all_alphas = all_alphas.into_inner().unwrap_or_default();
    let all_rf_models = all_rf_models.into_inner().unwrap_or_default();
    Ok((all_alphas, all_rf_models))
}

pub fn train_labels_for_all_algorithms(
    key: &str,
    sector: &str,
    labeled_data: Vec<(f64, Vec<f64>)>,
    n: i32,
    model_type: ModelType,
) -> Result<(Vec<TickerAlpha>, HashMap<String, RandomForestModel>)> {
    // debug!("  Period: {} Sector: {}", n, sector);
    let split_idx = (labeled_data.len() as f64 * 0.8) as usize;
    let (train_data, test_data) = labeled_data.split_at(split_idx);

    let train_size = train_data.len() as i32;
    let test_size = test_data.len() as i32;
    debug!(
        "          Training data: {} Test data: {}",
        train_size, test_size
    );
    // Normalization params from training set only
    // Stored in TickerAlpha and reused at inference via normalize_single
    let (means, stds) = compute_normalization_params(train_data);

    debug!("          Means sample: {:?}", &means[..5.min(means.len())]);
    debug!("          Stds  sample: {:?}", &stds[..5.min(stds.len())]);

    // Stats
    let sample_count = labeled_data.len() as i32;
    let feature_count = labeled_data[0].1.len() as i32;
    let up_count = labeled_data.iter().filter(|(l, _)| *l > 0.0).count() as i32;
    let down_count = labeled_data.iter().filter(|(l, _)| *l < 0.0).count() as i32;

    debug!(
        "          samples: {} UP: {} DOWN: {}",
        sample_count, up_count, down_count
    );

    let mut all_alphas = Vec::new();
    // let mut all_rf_models= HashMap::new();
    let mut all_rf_models = HashMap::new();

    // Train all three — same labeled data, different algorithm
    for model_algorithm in &[
        ModelAlgorithm::LinearRegression,
        ModelAlgorithm::RandomForest,
        ModelAlgorithm::MLP,
    ] {
        let means_clone = means.clone();
        let stds_clone = stds.clone();
        debug!("          Training model: {:?}...", model_algorithm);
        // Time-based split — never shuffle time series data
        let (
            metrics,
            intercept,
            params,
            rf_model,
            mean_down,
            mean_neutral,
            mean_up,
            mlp_weights,
            label_mean,
            label_std,
        ) = match model_algorithm {
            ModelAlgorithm::LinearRegression => {
                match train_models_for_linfa(
                    &labeled_data,
                    train_data,
                    test_data,
                    &means_clone,
                    &stds_clone,
                ) {
                    Ok(result) => (
                        result.metrics,
                        result.intercept,
                        result.params,
                        None,
                        0.0,
                        0.0,
                        0.0,
                        None,
                        0.0,
                        0.0,
                    ),
                    Err(e) => {
                        warn!(
                            "LR training failed for {} period {}: {}, skipping",
                            key, n, e
                        );
                        continue;
                    }
                }
            }

            ModelAlgorithm::RandomForest => {
                match train_models_for_randomforest(
                    &labeled_data,
                    train_data,
                    test_data,
                    &means_clone,
                    &stds_clone,
                ) {
                    Ok(result) => (
                        result.metrics,
                        0.0,
                        vec![],
                        Some(result.model),
                        result.mean_down,
                        result.mean_neutral,
                        result.mean_up,
                        None,
                        0.0,
                        0.0,
                    ),
                    Err(e) => {
                        warn!(
                            "RF training failed for {} period {}: {}, skipping",
                            key, n, e
                        );
                        continue;
                    }
                }
            }
            ModelAlgorithm::MLP => {
                match train_models_for_mlp(
                    &labeled_data,
                    train_data,
                    test_data,
                    &means_clone,
                    &stds_clone,
                ) {
                    Ok(result) => (
                        result.metrics,
                        0.0,
                        vec![],
                        None,
                        result.mean_down,
                        result.mean_neutral,
                        result.mean_up,
                        Some(result.weights),
                        result.label_mean,
                        result.label_std,
                    ),
                    Err(e) => {
                        warn!(
                            "MLP training failed for {} period {}: {}, skipping",
                            key, n, e
                        );
                        continue;
                    }
                }
            }
        };

        // if metrics.directional_accuracy < MIN_DIRECTIONAL_ACCURACY {
        //     warn!(
        //         "Ticker {} period {} accuracy too low: {:.1}%, skipping",
        //         key,
        //         n,
        //         metrics.directional_accuracy * 100.0
        //     );
        //     continue;
        // }

        let date = Utc::now();
        // let alpha_key = TickerAlpha::id(key, n, date);
        let alpha_key = TickerAlpha::new_id(key, n, model_algorithm);

        debug!(
            "              Key: {} Dir Acc: {:.1}%  Bullish: {:.1}%  Bearish: {:.1}%  MAE: {:.4}  R2: {:.4}",
            alpha_key,
            metrics.directional_accuracy * 100.0,
            metrics.bullish_precision * 100.0,
            metrics.bearish_precision * 100.0,
            metrics.mae,
            metrics.r2
        );

        let alpha = TickerAlpha {
            id: alpha_key.clone(),
            key: key.to_string(),
            n,
            date,
            sector: sector.to_string(),
            industry: String::new(),
            model_type: model_type.clone(),
            model_algorithm: model_algorithm.clone(),
            training_days: sample_count,
            feature_count,
            intercept,
            params,
            means: means_clone,
            stds: stds_clone,
            sample_count,
            bullish_count: up_count,
            bearish_count: down_count,
            train_size,
            test_size,
            directional_accuracy: metrics.directional_accuracy,
            bullish_precision: metrics.bullish_precision,
            bearish_precision: metrics.bearish_precision,
            mae: metrics.mae,
            r2: metrics.r2,
            mean_down,
            mean_neutral,
            mean_up,
            mlp_weights,
            label_mean,
            label_std,
        };

        all_alphas.push(alpha);
        if let Some(rf_model) = rf_model {
            all_rf_models.insert(format!("{}:{}", key, n), rf_model);
        }
    }

    Ok((all_alphas, all_rf_models))
}

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
