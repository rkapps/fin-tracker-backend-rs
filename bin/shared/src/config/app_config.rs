use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub llm: LlmConfig,
}

#[derive(Debug, Deserialize)]
pub struct LlmConfig {
    pub providers: Vec<ProviderConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    pub api_key_env: Option<String>,
    pub base_url_env: Option<String>,
    pub models: Vec<String>,
    pub default_model: String,
}