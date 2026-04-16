use anyhow::Result;
use std::{collections::HashMap, sync::Arc};

use fin_storage::service::StorageService;
use linfa_ensemble::EnsembleLearner;
use linfa_trees::DecisionTree;
use tokio::sync::RwLock;

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
            rf_models,
        }
    }

    pub async fn build_and_train_all(&self) -> Result<()> {
        Ok(())
    }
}
