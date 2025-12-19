use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub enum ApiProvider {
    OpenAI,
    #[default]
    OpenRouter,
}

impl ApiProvider {
    pub fn base_url(&self) -> &'static str {
        match self {
            ApiProvider::OpenAI => "https://api.openai.com/v1/chat/completions",
            ApiProvider::OpenRouter => "https://openrouter.ai/api/v1/chat/completions",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ApiProvider::OpenAI => "OpenAI",
            ApiProvider::OpenRouter => "OpenRouter",
        }
    }

    pub fn default_model(&self) -> &'static str {
        match self {
            ApiProvider::OpenAI => "gpt-4o-mini",
            ApiProvider::OpenRouter => "google/gemini-3-flash-preview",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub openai_api_key: String,
    #[serde(default)]
    pub openrouter_api_key: String,
    #[serde(default, rename = "api_key")]
    pub legacy_api_key: Option<String>,
    pub model: String,
    #[serde(default)]
    pub provider: ApiProvider,
    #[serde(default = "default_debounce")]
    pub debounce_ms: u64,
}

fn default_debounce() -> u64 {
    3000
}

impl Default for Config {
    fn default() -> Self {
        Self {
            openai_api_key: String::new(),
            openrouter_api_key: String::new(),
            legacy_api_key: None,
            model: "google/gemini-3-flash-preview".to_string(),
            provider: ApiProvider::OpenRouter,
            debounce_ms: 3000,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let mut cfg: Self = confy::load("grammy", "config").unwrap_or_default();

        // Backward-compat migration: older versions stored a single api_key.
        if cfg.openai_api_key.trim().is_empty() {
            if let Some(k) = cfg.legacy_api_key.clone() {
                if !k.trim().is_empty() {
                    cfg.openai_api_key = k;
                }
            }
        }

        cfg
    }

    pub fn save(&self) {
        let _ = confy::store("grammy", "config", self.clone());
    }

    pub fn api_key_for_provider(&self, provider: &ApiProvider) -> String {
        match provider {
            ApiProvider::OpenAI => self.openai_api_key.clone(),
            ApiProvider::OpenRouter => self.openrouter_api_key.clone(),
        }
    }
}
