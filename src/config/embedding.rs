use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub providers: HashMap<String, EmbeddingProvider>,
    pub default_provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingProvider {
    pub provider_type: EmbeddingProviderType,
    pub model: String,
    pub api_key: Option<String>,
    pub model_path: Option<String>, // For local models
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingProviderType {
    VertexAI,
    OpenAI,
    HuggingFace,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        let mut providers = HashMap::new();

        // Default Vertex AI provider
        providers.insert(
            "vertex".to_string(),
            EmbeddingProvider {
                provider_type: EmbeddingProviderType::VertexAI,
                model: "text-embedding-004".to_string(),
                api_key: None, // Uses GCP credentials
                model_path: None,
            },
        );

        // Default OpenAI provider
        providers.insert(
            "openai".to_string(),
            EmbeddingProvider {
                provider_type: EmbeddingProviderType::OpenAI,
                model: "text-embedding-3-small".to_string(),
                api_key: None, // Will be loaded from env
                model_path: None,
            },
        );

        // Default HuggingFace provider
        providers.insert(
            "huggingface".to_string(),
            EmbeddingProvider {
                provider_type: EmbeddingProviderType::HuggingFace,
                model: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
                api_key: None,
                model_path: None, // Will be downloaded
            },
        );

        Self {
            providers,
            default_provider: "vertex".to_string(),
        }
    }
}

impl EmbeddingConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let mut config = Self::default();

        // Update OpenAI provider with API key if available
        if let Ok(openai_key) = env::var("OPENAI_API_KEY") {
            if let Some(provider) = config.providers.get_mut("openai") {
                provider.api_key = Some(openai_key);
            }
        }

        // Update HuggingFace provider with API key if available
        if let Ok(hf_key) = env::var("HUGGINGFACE_API_KEY") {
            if let Some(provider) = config.providers.get_mut("huggingface") {
                provider.api_key = Some(hf_key);
            }
        }

        // Override default provider if specified
        if let Ok(default_provider) = env::var("DEFAULT_EMBEDDING_PROVIDER") {
            config.default_provider = default_provider;
        }

        // Load custom provider configurations from environment
        // Format: EMBEDDING_PROVIDER_<NAME>_TYPE, EMBEDDING_PROVIDER_<NAME>_MODEL, etc.
        for (key, value) in env::vars() {
            if key.starts_with("EMBEDDING_PROVIDER_") {
                let parts: Vec<&str> = key.split('_').collect();
                if parts.len() >= 4 {
                    let provider_name = parts[2].to_lowercase();
                    let config_key = parts[3].to_lowercase();

                    let provider = config.providers.entry(provider_name.clone()).or_insert(
                        EmbeddingProvider {
                            provider_type: EmbeddingProviderType::VertexAI,
                            model: "".to_string(),
                            api_key: None,
                            model_path: None,
                        },
                    );

                    match config_key.as_str() {
                        "type" => {
                            provider.provider_type = match value.to_lowercase().as_str() {
                                "openai" => EmbeddingProviderType::OpenAI,
                                "huggingface" => EmbeddingProviderType::HuggingFace,
                                _ => EmbeddingProviderType::VertexAI,
                            };
                        }
                        "model" => provider.model = value,
                        "api_key" => provider.api_key = Some(value),
                        "model_path" => provider.model_path = Some(value),
                        _ => {}
                    }
                }
            }
        }

        Ok(config)
    }
}
