use anyhow::Result;
use chrono::{DateTime, Months, Utc};
use fin_domain::ticker::{
    FeatureSnapshot, IndicatorSnapshot, ModelAlgorithm, ModelType, Ticker, TickerAlpha,
    TickerIndicator,
};
use fin_storage::service::StorageService;
use linfa_ensemble::EnsembleLearner;
use linfa_trees::DecisionTree;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{
    collections::HashMap,
    panic,
    sync::{Arc, Mutex},
    vec,
};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::ml::{
    labels::{build_labels_for_ticker, compute_normalization_params, normalize_single},
    linfa_lr::{run_lr_predictions, train_models_for_linfa},
    linfa_rf::{run_rf_predictions, train_models_for_randomforest},
    mlp::{predict_mlp, train_models_for_mlp},
};

const PERIODS: [i32; 4] = [5, 10, 20, 60];
// const PERIODS: [i32; 2] = [10, 60];
const MIN_SAMPLES: usize = 100;
// Don't store alphas below this threshold — useless at prediction time
// const MIN_DIRECTIONAL_ACCURACY: f64 = 0.40;

#[derive(Clone)]
pub struct MlService {
    pub storage_service: Arc<dyn StorageService>,
    rf_models: Arc<RwLock<HashMap<String, Option<EnsembleLearner<DecisionTree<f64, usize>>>>>>,
}
impl std::fmt::Debug for MlService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MlService").finish()
    }
}

impl MlService {
    pub fn new(storage_service: Arc<dyn StorageService>) -> Self {
        let rf_models = Arc::new(RwLock::new(HashMap::new()));
        Self {
            storage_service,
            rf_models: rf_models,
        }
    }

    // ── Public entry points ───────────────────────────────────────────────

    pub async fn build_and_train_all(&self) -> Result<()> {
        // let result = self.build_and_train_by_sector(from_date).await?;
        // let _ = self.storage_service.save_ticker_alphas(&result.0).await;
        // for value in result.1 {
        //     let mut lock = self.rf_models.write().await;
        //     lock.insert(value.0, value.1);
        // }

        let from_date = Utc::now().checked_sub_months(Months::new(60)).unwrap();
        info!("Training sector models...");

        info!("Training per-ticker models...");
        let result = self.build_and_train_by_ticker(from_date).await?;
        let _ = self.storage_service.save_ticker_alphas(&result.0).await;
        for value in result.1 {
            let mut lock = self.rf_models.write().await;
            lock.insert(value.0, value.1);
        }

        Ok(())
    }

    pub async fn build_and_train_by_sector(
        &self,
        from_date: DateTime<Utc>,
    ) -> Result<(
        Vec<TickerAlpha>,
        HashMap<String, Option<EnsembleLearner<DecisionTree<f64, usize>>>>,
    )> {
        let groups = self.storage_service.get_ticker_groups().await?;
        let mut all_alphas = Vec::new();
        let mut all_rf_models = HashMap::new();

        for group in groups {
            // if group.0 != "Technology".to_string() {
            //     continue;
            // }
            let indicators_map = self
                .storage_service
                .get_ticker_indicators_map_by_sector(&group.0, from_date)
                .await?;

            info!("Sector: {} tickers: {}", group.0, indicators_map.len());
            for n in &PERIODS {
                // Build labels per ticker, append all into one sector dataset
                let mut sector_labels: Vec<(f64, Vec<f64>)> = Vec::new();

                for (symbol, indicators) in &indicators_map {
                    if indicators.len() < MIN_SAMPLES {
                        warn!(
                            "Ticker {} insufficient data: {} rows",
                            symbol,
                            indicators.len()
                        );
                        continue;
                    }

                    // if !(symbol == "NVDA" || symbol == "AAPL") {
                    //     continue;
                    // }
                    let ticker_labels = build_labels_for_ticker(indicators, *n as usize)?;
                    debug!("Ticker: {} labels: {}", symbol, ticker_labels.len());
                    sector_labels.extend(ticker_labels);
                }
                let result = Self::train_labels_for_all_algorithms(
                    &group.0,
                    &group.0,
                    sector_labels.clone(),
                    *n,
                    ModelType::Sector,
                )?;
                all_alphas.extend(result.0);
                for value in result.1 {
                    all_rf_models.insert(value.0, value.1);
                }
            }
        }

        Ok((all_alphas, all_rf_models))
    }

    pub async fn build_and_train_by_ticker(
        &self,
        from_date: DateTime<Utc>,
    ) -> Result<(
        Vec<TickerAlpha>,
        HashMap<String, Option<EnsembleLearner<DecisionTree<f64, usize>>>>,
    )> {
        let tickers = self.storage_service.get_tickers_by_marketcap().await?;

        let length = tickers.len();

        // --- Step 1: Fetch all indicator data async (sequential, IO-bound) ---
        let mut ticker_data: Vec<(Ticker, Vec<TickerIndicator>)> = Vec::new();

        for (i, ticker) in tickers.iter().enumerate() {
            if i % 20 == 0 {
                info!("Fetching Ticker: {} {}/{}", ticker.symbol, i + 1, length);
            }

            let indicators = self
                .storage_service
                .get_ticker_indicators_by_symbol(&ticker.symbol, from_date)
                .await?;

            if indicators.len() < MIN_SAMPLES {
                warn!(
                    "Ticker {} insufficient data: {} rows",
                    ticker.symbol,
                    indicators.len()
                );
                continue;
            }

            ticker_data.push((ticker.clone(), indicators));
        }

        info!(
            "Fetched {} tickers, starting parallel training...",
            ticker_data.len()
        );

        // --- Step 2: Parallel training (CPU-bound) ---
        let all_alphas = Mutex::new(Vec::new());
        let all_rf_models = Mutex::new(HashMap::new());

        ticker_data.par_iter().for_each(|(ticker, indicators)| {
            let mut ticker_alphas = Vec::new();
            let mut ticker_rf_models = HashMap::new();
            let mut success_count = 0;

            for n in &PERIODS {
                let ticker_labels = match build_labels_for_ticker(indicators, *n as usize) {
                    Ok(labels) => labels,
                    Err(e) => {
                        warn!("Label build failed {} period {}: {}", ticker.symbol, n, e);
                        continue;
                    }
                };

                match Self::train_labels_for_all_algorithms(
                    &ticker.symbol,
                    &ticker.sector.clone().unwrap_or_default(),
                    ticker_labels,
                    *n,
                    ModelType::Ticker,
                ) {
                    Ok(result) => {
                        ticker_alphas.extend(result.0);
                        ticker_rf_models.extend(result.1);
                        success_count += 1;
                    }
                    Err(e) => {
                        warn!("Training failed {} period {}: {}", ticker.symbol, n, e);
                    }
                }
            }

            info!(
                "  {} — trained {}/{} periods, {} alphas",
                ticker.symbol,
                success_count,
                PERIODS.len(),
                ticker_alphas.len()
            );

            if !ticker_alphas.is_empty() {
                match all_alphas.lock() {
                    Ok(mut guard) => guard.extend(ticker_alphas),
                    Err(e) => warn!("Mutex poisoned for {}: {}", ticker.symbol, e),
                }
            }

            if !ticker_rf_models.is_empty() {
                match all_rf_models.lock() {
                    Ok(mut guard) => guard.extend(ticker_rf_models),
                    Err(e) => warn!("RF mutex poisoned for {}: {}", ticker.symbol, e),
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
    ) -> Result<(
        Vec<TickerAlpha>,
        HashMap<String, Option<EnsembleLearner<DecisionTree<f64, usize>>>>,
    )> {
        debug!("  Period: {} Sector: {}", n, sector);
        let split_idx = (labeled_data.len() as f64 * 0.8) as usize;
        let (train_data, test_data) = labeled_data.split_at(split_idx);

        let train_size = train_data.len() as i32;
        let test_size = test_data.len() as i32;

        // Normalization params from training set only
        // Stored in TickerAlpha and reused at inference via normalize_single
        let (means, stds) = compute_normalization_params(train_data);

        debug!("  Means sample: {:?}", &means[..5.min(means.len())]);
        debug!("  Stds  sample: {:?}", &stds[..5.min(stds.len())]);

        // Stats
        let sample_count = labeled_data.len() as i32;
        let feature_count = labeled_data[0].1.len() as i32;
        let up_count = labeled_data.iter().filter(|(l, _)| *l > 0.0).count() as i32;
        let down_count = labeled_data.iter().filter(|(l, _)| *l < 0.0).count() as i32;

        debug!(
            "  samples: {} UP: {} DOWN: {}",
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
            debug!("  Training model: {:?}...", model_algorithm);
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

            info!(
                "Key: {} Dir Acc: {:.1}%  Bullish: {:.1}%  Bearish: {:.1}%  MAE: {:.4}  R2: {:.4}",
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
                training_days: sample_count as i32,
                feature_count,
                intercept: intercept,
                params: params,
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
            all_rf_models.insert(format!("{}:{}", key, n), rf_model);
        }

        Ok((all_alphas, all_rf_models))
    }

    // ── Predictions ───────────────────────────────────────────────────────

    pub async fn run_predictions(
        &self,
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
                    let lock = self.rf_models.read().await;
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

    /// Prefer ticker model, fall back to sector model
    pub async fn run_ticker_predictions(
        &self,
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

        self.run_predictions(indicator, prev_indicator, sas.to_vec())
            .await
    }
}
