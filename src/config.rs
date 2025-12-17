use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
pub enum ApiProvider {
    #[default]
    OpenAI,
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
            ApiProvider::OpenRouter => "openai/gpt-4o-mini",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub api_key: String,
    pub model: String,
    #[serde(default)]
    pub provider: ApiProvider,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: "gpt-4o-mini".to_string(),
            provider: ApiProvider::OpenAI,
        }
    }
}

impl Config {
    pub fn load() -> Self {
        confy::load("grammy", "config").unwrap_or_default()
    }

    pub fn save(&self) {
        let _ = confy::store("grammy", "config", self.clone());
    }
}
