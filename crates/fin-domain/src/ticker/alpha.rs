use chrono::{DateTime, Utc};
use fin_providers::alpha::model;
use serde::{Deserialize, Serialize};
use storage_core::core::RepoModel;

use crate::ticker::TICKER_ALPHA_COLLECTION_NAME;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ModelType {
    Sector,
    Ticker,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub enum ModelAlgorithm {
    LinearRegression,
    RandomForest,
    MLP,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TickerAlpha {
    pub id: String,
    pub key: String,
    pub n: i32,
    pub date: DateTime<Utc>,
    pub model_type: ModelType,
    pub model_algorithm: ModelAlgorithm,
    pub sector: String,
    pub industry: String,
    pub training_days: i32,
    pub feature_count: i32,
    pub intercept: f64,
    pub params: Vec<f64>,
    pub means: Vec<f64>,
    pub stds: Vec<f64>,
    pub sample_count: i32,
    pub bullish_count: i32,
    pub bearish_count: i32,
    pub train_size: i32,
    pub test_size: i32,
    pub directional_accuracy: f64,
    pub bullish_precision: f64,
    pub bearish_precision: f64,
    pub mae: f64,
    pub r2: f64,
    pub mean_up:      f64,
    pub mean_neutral: f64,
    pub mean_down:    f64,

    //mlp_weights
    pub label_mean: f64,
    pub label_std: f64,
    pub mlp_weights: Option<Vec<u8>>,

}


impl RepoModel<String> for TickerAlpha {
    fn id(&self) -> String {
        self.id.clone()
    }

    fn collection(&self) -> &'static str {
        TICKER_ALPHA_COLLECTION_NAME
    }
}

impl TickerAlpha {
    pub fn id(sector: &str, n: i32, date: DateTime<Utc>) -> String {
        format!("{}:{}:{}", sector, n, date.timestamp_millis())
    }
     pub fn new_id(sector: &str, n: i32, model_algorithm: &ModelAlgorithm) -> String {
        format!("{}:{}:{:?}", sector, n, model_algorithm)
    }
}