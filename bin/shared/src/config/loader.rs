use anyhow::Result;
use tracing::info;

use crate::{config::app_config::AppConfig, gcs::download_gcs_string};

pub async fn load_app_config() -> Result<AppConfig> {
    let config_path =
        std::env::var("APP_CONFIG_PATH").unwrap_or_else(|_| "bin/shared/src/config/app_config.json".to_string());

    info!("App Config Path: {}", config_path);
    let content = if config_path.starts_with("gs://") {
        download_gcs_string(&config_path).await?
    } else {
        std::fs::read_to_string(&config_path)?
    };

    Ok(serde_json::from_str(&content)?)
}
