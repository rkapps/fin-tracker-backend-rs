use linfa_ensemble::EnsembleLearner;
use linfa_trees::DecisionTree;

pub struct LrResult {
    pub intercept: f64,
    pub params: Vec<f64>,
    pub metrics: ModelMetrics,
}

pub struct RfResult {
    pub model: EnsembleLearner<DecisionTree<f64, usize>>,
    pub metrics: ModelMetrics,
    pub mean_up: f64,
    pub mean_neutral: f64,
    pub mean_down: f64,
}

pub struct ModelMetrics {
    pub directional_accuracy: f64,
    pub bullish_precision: f64,
    pub bearish_precision: f64,
    pub mae: f64,
    pub r2: f64,
}
