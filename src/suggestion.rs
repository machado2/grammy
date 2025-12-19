use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: String,
    pub message: String,
    pub offset: usize,
    pub length: usize,
    pub original: String,
    pub replacement: Option<String>,
}

impl Suggestion {
    pub fn new(
        message: String,
        offset: usize,
        original: String,
        replacement: Option<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            message,
            offset,
            length: original.len(),
            original,
            replacement,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMatch {
    pub message: String,
    pub original: String,
    pub replacement: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub matches: Vec<LlmMatch>,
}
