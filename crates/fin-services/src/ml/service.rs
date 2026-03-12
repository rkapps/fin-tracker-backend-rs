use anyhow::Result;
use chrono::{DateTime, Months, Utc};
use fin_domain::ticker::{
    FeatureSnapshot, IndicatorSnapshot, ModelAlgorithm, ModelType, TickerAlpha, TickerIndicator,
};
use fin_storage::service::StorageService;
use linfa_ensemble::EnsembleLearner;
use linfa_trees::DecisionTree;
use std::{collections::HashMap, sync::Arc, vec};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::ml::{
    labels::{build_labels_for_ticker, compute_normalization_params, normalize_single},
    linfa_lr::{run_prediction_for_lr, train_models_for_linfa},
    linfa_rf::{run_prediction_for_rf, train_models_for_randomforest},
};

const PERIODS: [i32; 4] = [5, 10, 20, 60];
const MIN_SAMPLES: usize = 200;
// Don't store alphas below this threshold — useless at prediction time
const MIN_DIRECTIONAL_ACCURACY: f64 = 0.40;

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
        let from_date = Utc::now().checked_sub_months(Months::new(36)).unwrap();
        // let from_date = Utc::now().checked_sub_days(Days::new(40)).unwrap();

        info!("Training sector models...");

        // let result = self.build_and_train_by_sector(from_date).await?;
        // let _ = self.storage_service.save_ticker_alphas(&result.0).await;
        // for value in result.1 {
        //     let mut lock = self.rf_models.write().await;
        //     lock.insert(value.0, value.1);
        // }

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
        let tickers = self.storage_service.get_tickers().await?;

        let mut all_alphas = Vec::new();
        let mut all_rf_models = HashMap::new();
        let length = tickers.len();
        for (i, ticker) in tickers.iter().enumerate() {
            // for ticker in &tickers {
            if !(ticker.symbol == "NVDA" || ticker.symbol == "AAPL") {
                continue;
            }
            if i % 20 == 0 {
                info!("Training Ticker: {} {}/{}", ticker.symbol, i + 1, length);
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

            for n in &PERIODS {
                // Same function — single ticker, first record skipped
                let ticker_labels = build_labels_for_ticker(&indicators, *n as usize)?;
                info!("Ticker: {} labels: {}", ticker.symbol, ticker_labels.len());
                let result = Self::train_labels_for_all_algorithms(
                    &ticker.symbol,
                    &ticker.sector.clone().unwrap(),
                    ticker_labels.clone(),
                    *n,
                    ModelType::Ticker,
                )?;

                all_alphas.extend(result.0);
                for value in result.1 {
                    all_rf_models.insert(value.0, value.1);
                }
            }
        }

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
        info!("Key: {} Sector: {}", key, sector);
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

        info!(
            "  Period: {} samples: {} UP: {} DOWN: {}",
            n, sample_count, up_count, down_count
        );

        let mut all_alphas = Vec::new();
        // let mut all_rf_models= HashMap::new();
        let mut all_rf_models = HashMap::new();

        // Train all three — same labeled data, different algorithm
        for model_algorithm in &[
            ModelAlgorithm::LinearRegression,
            ModelAlgorithm::RandomForest,
            // ModelAlgorithm::MLP,
        ] {
            let means_clone = means.clone();
            let stds_clone = stds.clone();
            info!("  Training model: {:?}...", model_algorithm);
            // Time-based split — never shuffle time series data
            let (metrics, intercept, params, rf_model, mean_down, mean_neutral, mean_up) =
                match model_algorithm {
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
                            Ok(result) => {
                                (
                                    result.metrics,
                                    0.0,
                                    vec![],
                                    Some(result.model),
                                    result.mean_down,
                                    result.mean_neutral,
                                    result.mean_up,
                               )
       
                            },
                            Err(e) => {
                                warn!(
                                    "RF training failed for {} period {}: {}, skipping",
                                    key, n, e
                                );
                                continue;
                            }
                        }
                    }

                };

            if metrics.directional_accuracy < MIN_DIRECTIONAL_ACCURACY {
                warn!(
                    "Ticker {} period {} accuracy too low: {:.1}%, skipping",
                    key,
                    n,
                    metrics.directional_accuracy * 100.0
                );
                continue;
            }

            let date = Utc::now();
            let alpha_key = TickerAlpha::id(key, n, date);
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
    ) -> Result<(HashMap<String, f64>, HashMap<String, f64>)> {
        let isnapshot = IndicatorSnapshot::from(indicator);
        let prev_snapshot = prev_indicator.map(|p| IndicatorSnapshot::from(p));

        let fsnapshot =
            FeatureSnapshot::from_indicator_with_prev(&isnapshot, prev_snapshot.as_ref())?;

        let mut lf_returns = HashMap::new();
        let mut rf_returns = HashMap::new();

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

            match sa.model_algorithm {
                ModelAlgorithm::LinearRegression => {
                    let predicted_return = run_prediction_for_lr(&sa, &normalized);
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
                        warn!("RF model not found for {}", sa.id);
                        continue;
                    };
                    match run_prediction_for_rf(&sa, model, normalized) {
                        Ok(c) => {
                            rf_returns.insert(sa.n.to_string(), c);
                            debug!(
                                "Period {} algorithm: {:?} predicted: {:.2}%",
                                sa.id, sa.model_algorithm, c
                            );
                        }
                        Err(e) => {
                            return Err(anyhow::anyhow!("Prediction error: {}", e));
                        }
                    };
                }
            };
        }

        Ok((lf_returns, rf_returns))
    }

    /// Prefer ticker model, fall back to sector model
    pub async fn run_ticker_predictions(
        &self,
        indicator: &TickerIndicator,
        prev_indicator: Option<&TickerIndicator>,
        ticker_alphas: Vec<TickerAlpha>,
        sector_alphas: Vec<TickerAlpha>,
    ) -> Result<(HashMap<String, f64>, HashMap<String, f64>)> {
        let sas = if !ticker_alphas.is_empty() {
            debug!("Using ticker model for {}", indicator.symbol);
            ticker_alphas
        } else {
            debug!("Falling back to sector model for {}", indicator.symbol);
            sector_alphas
        };

        self.run_predictions(indicator, prev_indicator, sas).await
    }
}
