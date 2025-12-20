use serde::{Deserialize, Serialize};

/// Severity level for a suggestion, determines highlighting color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    #[default]
    Error, // Red - grammar errors, typos
    Warning,    // Orange - awkward phrasing
    Suggestion, // Yellow - minor improvements
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: String,
    pub message: String,
    pub offset: usize,
    pub length: usize,
    pub original: String,
    pub replacement: Option<String>,
    pub severity: Severity,
}

impl Suggestion {
    pub fn new(
        message: String,
        offset: usize,
        original: String,
        replacement: Option<String>,
        severity: Severity,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            message,
            offset,
            length: original.len(),
            original,
            replacement,
            severity,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMatch {
    pub message: String,
    pub original: String,
    pub replacement: Option<String>,
    #[serde(default)]
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub matches: Vec<LlmMatch>,
}
