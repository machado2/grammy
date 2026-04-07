use crate::app::history::HistoryEntry;
use crate::config::{ApiProvider, ReasoningEffort};
use crate::suggestion::{LlmMatch, LlmResponse, Suggestion};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);
const RESPONSE_BODY_TIMEOUT_SECS: u64 = 90;

const SYSTEM_PROMPT: &str = r#"You are a strict English grammar checker.
Your job: suggest edits ONLY for clear, objective errors:
1. Grammatical errors.
2. Typos.
3. Clearly incorrect word forms or agreement errors.

Rules:
- Do NOT suggest stylistic improvements.
- Do NOT suggest rephrasing for awkwardness, tone, fluency, or naturalness.
- Do NOT rewrite the text.
- Do NOT suggest optional punctuation changes unless the original is clearly incorrect.
- If a sentence is grammatically correct, do NOT suggest anything.
- Only return matches when there is a specific incorrect substring and a specific correction.
- Always provide a concrete replacement for each match.

Return ONLY valid JSON with this exact schema:
{
  "matches": [
    {
      "message": "explanation of the error",
      "original": "exact text to replace",
      "replacement": "corrected text",
      "severity": "error"
    }
  ]
}

IMPORTANT: The "original" field must contain the EXACT substring from the input (copy it precisely, including spacing).
If there is nothing to change, return {"matches": []}."#;

fn log_preview(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

pub async fn check_grammar(
    client: &reqwest::Client,
    text: String,
    api_key: String,
    model: String,
    provider: ApiProvider,
    reasoning_effort: ReasoningEffort,
    request_id: u64,
    history: Vec<HistoryEntry>,
) -> Result<(Vec<Suggestion>, u64), String> {
    let start = Instant::now();
    eprintln!(
        "[DEBUG #{request_id}] Starting grammar check, provider={}, model={}, text_len={}",
        provider.name(),
        model,
        text.len()
    );

    if api_key.is_empty() {
        eprintln!("[DEBUG #{request_id}] Error: API key not set");
        return Err("API key not set. Click ⚙ to configure.".to_string());
    }

    if text.trim().is_empty() {
        eprintln!("[DEBUG #{request_id}] Empty text, returning no suggestions");
        return Ok((vec![], request_id));
    }

    // Build messages array: system prompt + history + current user message
    let mut messages = vec![json!({ "role": "system", "content": SYSTEM_PROMPT })];

    // Add history entries (user/assistant pairs)
    for entry in &history {
        messages.push(json!({
            "role": entry.role,
            "content": entry.content
        }));
    }

    // Add current user message
    messages.push(json!({
        "role": "user",
        "content": format!("Text:\n{}", text)
    }));

    let url = if provider == ApiProvider::Gemini {
        format!(
            "{}{}:generateContent?key={}",
            provider.base_url(),
            model,
            api_key
        )
    } else {
        provider.base_url().to_string()
    };

    eprintln!("[DEBUG #{request_id}] Sending request to {}", url);

    let mut request = client.post(&url).header("Content-Type", "application/json");

    if provider == ApiProvider::Gemini {
        let body = json!({
            "contents": [{
                "parts": [{
                    "text": format!("{}\n\nText:\n{}", SYSTEM_PROMPT, text)
                }]
            }],
            "generationConfig": {
                "responseMimeType": "application/json"
            }
        });
        request = request.json(&body);
    } else {
        let mut body = json!({
            "model": model,
            "messages": messages,
            "response_format": { "type": "json_object" },
            "stream": false
        });

        if let Some(effort) = reasoning_effort.as_api_str() {
            match provider {
                ApiProvider::OpenAI => {
                    body["reasoning_effort"] = json!(effort);
                }
                ApiProvider::OpenRouter => {
                    body["reasoning"] = json!({ "effort": effort });
                }
                ApiProvider::Gemini => {}
            }
        }

        request = request
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body);

        // Add OpenRouter-specific headers
        if provider == ApiProvider::OpenRouter {
            request = request
                .header("HTTP-Referer", "https://github.com/grammy-app")
                .header("X-Title", "Grammy");
        }
    }

    let response = request.send().await.map_err(|e| {
        eprintln!(
            "[DEBUG #{request_id}] Network error after {:?}: {}",
            start.elapsed(),
            e
        );
        format!("Network error: {}", e)
    })?;

    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("<unknown>");
    let content_length = response.content_length();
    eprintln!(
        "[DEBUG #{request_id}] Response status: {} after {:?} (content_type={}, content_length={:?})",
        status,
        start.elapsed(),
        content_type,
        content_length
    );

    eprintln!(
        "[DEBUG #{request_id}] Reading response body with timeout={}s",
        RESPONSE_BODY_TIMEOUT_SECS
    );
    let body_bytes = match tokio::time::timeout(
        Duration::from_secs(RESPONSE_BODY_TIMEOUT_SECS),
        response.bytes(),
    )
    .await
    {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(e)) => {
            eprintln!("[DEBUG #{request_id}] Failed reading response body: {}", e);
            return Err(format!("Failed reading response body: {}", e));
        }
        Err(_) => {
            eprintln!(
                "[DEBUG #{request_id}] Timed out reading response body after {}s",
                RESPONSE_BODY_TIMEOUT_SECS
            );
            return Err(format!(
                "Timed out reading response body after {}s",
                RESPONSE_BODY_TIMEOUT_SECS
            ));
        }
    };
    eprintln!(
        "[DEBUG #{request_id}] Response body read: {} bytes",
        body_bytes.len()
    );

    let data: serde_json::Value = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(e) => {
            let body_preview = log_preview(String::from_utf8_lossy(&body_bytes).as_ref(), 300);
            if !status.is_success() {
                eprintln!(
                    "[DEBUG #{request_id}] Non-JSON error response body (status={}): {}",
                    status, body_preview
                );
                return Err(format!(
                    "{} error ({}): {}",
                    provider.name(),
                    status,
                    body_preview
                ));
            }

            eprintln!(
                "[DEBUG #{request_id}] Failed to parse response JSON: {} | body preview: {}",
                e, body_preview
            );
            return Err(format!("Failed to parse response JSON: {}", e));
        }
    };

    if !status.is_success() {
        let msg = if provider == ApiProvider::Gemini {
            data["error"]["message"]
                .as_str()
                .unwrap_or("Unknown Gemini error")
        } else {
            data["error"]["message"].as_str().unwrap_or("Unknown error")
        };
        eprintln!("[DEBUG #{request_id}] API error: {} - {}", status, msg);
        return Err(format!("{} error ({}): {}", provider.name(), status, msg));
    }

    let content = if provider == ApiProvider::Gemini {
        data["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or(r#"{"matches":[]}"#)
    } else {
        data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or(r#"{"matches":[]}"#)
    };

    if content == r#"{"matches":[]}"# {
        eprintln!(
            "[DEBUG #{request_id}] Response content path missing/empty for provider={}, falling back to empty matches",
            provider.name()
        );
    }

    eprintln!(
        "[DEBUG #{request_id}] LLM response content: {}",
        log_preview(content, 200)
    );

    let llm_response: LlmResponse = serde_json::from_str(content).map_err(|e| {
        eprintln!("[DEBUG #{request_id}] Invalid JSON from LLM: {}", e);
        format!("Invalid JSON from LLM: {}", e)
    })?;

    let suggestions = convert_matches_to_suggestions(&text, llm_response.matches);
    eprintln!(
        "[DEBUG #{request_id}] Completed in {:?}, found {} suggestions",
        start.elapsed(),
        suggestions.len()
    );

    Ok((suggestions, request_id))
}

pub fn next_request_id() -> u64 {
    REQUEST_COUNTER.fetch_add(1, Ordering::SeqCst)
}

pub async fn test_connection(
    client: &reqwest::Client,
    api_key: String,
    provider: ApiProvider,
    model: String,
    request_id: u64,
) -> Result<u64, String> {
    let start = Instant::now();
    eprintln!(
        "[DEBUG #{request_id}] Starting connection test, provider={}, model={}",
        provider.name(),
        model
    );

    if api_key.is_empty() {
        eprintln!("[DEBUG #{request_id}] Error: API key not set");
        return Err("API key not set. Click ⚙ to configure.".to_string());
    }

    let (url, is_post) = match provider {
        ApiProvider::OpenAI => ("https://api.openai.com/v1/models".to_string(), false),
        ApiProvider::OpenRouter => ("https://openrouter.ai/api/v1/key".to_string(), false),
        ApiProvider::Gemini => (
            format!(
                "https://generativelanguage.googleapis.com/v1beta/models?key={}",
                api_key
            ),
            false,
        ),
    };

    eprintln!("[DEBUG #{request_id}] Sending test request to {}", url);

    let mut request = if is_post {
        client.post(&url)
    } else {
        client.get(&url)
    };

    if provider != ApiProvider::Gemini {
        request = request.header("Authorization", format!("Bearer {}", api_key));
    }

    if provider == ApiProvider::OpenRouter {
        request = request
            .header("HTTP-Referer", "https://github.com/grammy-app")
            .header("X-Title", "Grammy");
    }

    let response = request.send().await.map_err(|e| {
        eprintln!(
            "[DEBUG #{request_id}] Network error after {:?}: {}",
            start.elapsed(),
            e
        );
        format!("Network error: {}", e)
    })?;

    let status = response.status();
    eprintln!(
        "[DEBUG #{request_id}] Test response status: {} after {:?}",
        status,
        start.elapsed()
    );

    if !status.is_success() {
        let msg = match response.json::<serde_json::Value>().await {
            Ok(v) => v
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .map(|s| s.to_string())
                .or_else(|| {
                    v.get("message")
                        .and_then(|m| m.as_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| v.to_string()),
            Err(_) => "Unauthorized".to_string(),
        };
        eprintln!("[DEBUG #{request_id}] Test API error: {} - {}", status, msg);
        return Err(format!("{} error ({}): {}", provider.name(), status, msg));
    }

    // If we're here, connection is OK. Now validate model if provided and not Gemini (which lists models already)
    // Actually, let's just check if the model is in the list of models for the provider.
    let models = fetch_models(client, provider.clone(), api_key, request_id).await?;
    if !model.is_empty() && !models.iter().any(|m| m == &model) {
        return Err(format!(
            "Model '{}' not found for {}",
            model,
            provider.name()
        ));
    }

    eprintln!(
        "[DEBUG #{request_id}] Connection test succeeded in {:?}",
        start.elapsed()
    );
    Ok(request_id)
}

pub async fn fetch_models(
    client: &reqwest::Client,
    provider: ApiProvider,
    api_key: String,
    request_id: u64,
) -> Result<Vec<String>, String> {
    let start = Instant::now();
    if api_key.is_empty() {
        eprintln!(
            "[DEBUG #{request_id}] Skipping models fetch because API key is empty (provider={})",
            provider.name()
        );
        return Ok(vec![]);
    }

    eprintln!(
        "[DEBUG #{request_id}] Fetching models for provider={}",
        provider.name()
    );

    let url = match provider {
        ApiProvider::OpenAI => "https://api.openai.com/v1/models".to_string(),
        ApiProvider::OpenRouter => "https://openrouter.ai/api/v1/models".to_string(),
        ApiProvider::Gemini => format!(
            "https://generativelanguage.googleapis.com/v1beta/models?key={}",
            api_key
        ),
    };

    let mut request = client.get(&url);
    if provider != ApiProvider::Gemini {
        request = request.header("Authorization", format!("Bearer {}", api_key));
    }

    let response = request.send().await.map_err(|e| {
        eprintln!(
            "[DEBUG #{request_id}] Models fetch network error after {:?}: {}",
            start.elapsed(),
            e
        );
        format!("Network error: {}", e)
    })?;

    let status = response.status();
    eprintln!(
        "[DEBUG #{request_id}] Models response status: {} after {:?}",
        status,
        start.elapsed()
    );

    if !status.is_success() {
        let body = response.text().await.unwrap_or_else(|err| {
            eprintln!(
                "[DEBUG #{request_id}] Failed to read models error body: {}",
                err
            );
            "<unavailable>".to_string()
        });
        eprintln!(
            "[DEBUG #{request_id}] Models fetch failed: {} - {}",
            status,
            log_preview(&body, 200)
        );
        return Err(format!("Failed to fetch models: {}", status));
    }

    let data: serde_json::Value = response.json().await.map_err(|e| {
        eprintln!(
            "[DEBUG #{request_id}] Failed to parse models response after {:?}: {}",
            start.elapsed(),
            e
        );
        format!("Failed to parse models response: {}", e)
    })?;
    let mut models = Vec::new();

    match provider {
        ApiProvider::OpenAI | ApiProvider::OpenRouter => {
            if let Some(data_array) = data["data"].as_array() {
                for m in data_array {
                    if let Some(id) = m["id"].as_str() {
                        models.push(id.to_string());
                    }
                }
            }
        }
        ApiProvider::Gemini => {
            if let Some(models_array) = data["models"].as_array() {
                for m in models_array {
                    if let Some(name) = m["name"].as_str() {
                        // Gemini returns "models/gemini-pro", we want just "gemini-pro"
                        let name = name.strip_prefix("models/").unwrap_or(name);
                        models.push(name.to_string());
                    }
                }
            }
        }
    }

    models.sort();
    eprintln!(
        "[DEBUG #{request_id}] Models fetch succeeded in {:?} (count={})",
        start.elapsed(),
        models.len()
    );
    Ok(models)
}

fn convert_matches_to_suggestions(text: &str, matches: Vec<LlmMatch>) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();

    for m in matches {
        if m.severity != crate::suggestion::Severity::Error {
            continue;
        }

        if m.original.is_empty() {
            continue;
        }

        let Some(ref repl) = m.replacement else {
            continue;
        };

        if repl.is_empty() || repl == &m.original {
            continue;
        }

        let offset = if let Some(pos) = text.find(&m.original) {
            pos
        } else {
            // Try case-insensitive search
            let lower_text = text.to_lowercase();
            let lower_original = m.original.to_lowercase();
            if let Some(pos) = lower_text.find(&lower_original) {
                pos
            } else {
                continue;
            }
        };

        suggestions.push(Suggestion::new(
            m.message,
            offset,
            m.original,
            m.replacement,
            m.severity,
        ));
    }

    suggestions.sort_by_key(|s| s.offset);

    // Filter overlapping suggestions
    let mut filtered = Vec::new();
    let mut last_end = 0;
    for s in suggestions {
        let end = s.offset + s.length;
        if s.offset < last_end {
            continue;
        }
        last_end = end;
        filtered.push(s);
    }

    filtered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suggestion::Severity;

    #[test]
    fn test_normal_suggestion() {
        let text = "I has a cat.";
        let matches = vec![LlmMatch {
            message: "grammar error".to_string(),
            original: "has".to_string(),
            replacement: Some("have".to_string()),
            severity: Severity::Error,
        }];

        let suggestions = convert_matches_to_suggestions(text, matches);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].original, "has");
        assert_eq!(suggestions[0].replacement, Some("have".to_string()));
    }

    #[test]
    fn test_comment_only_suggestion() {
        let text = "I has a cat.";
        let matches = vec![LlmMatch {
            message: "ambiguous phrasing".to_string(),
            original: "has".to_string(),
            replacement: None,
            severity: Severity::Warning,
        }];

        let suggestions = convert_matches_to_suggestions(text, matches);
        assert_eq!(suggestions.len(), 0);
    }

    #[test]
    fn test_empty_replacement_ignored() {
        let text = "I has a cat.";
        let matches = vec![LlmMatch {
            message: "test".to_string(),
            original: "has".to_string(),
            replacement: Some("".to_string()), // Should be ignored as invalid "replacement"
            severity: Severity::Error,
        }];

        let suggestions = convert_matches_to_suggestions(text, matches);
        assert_eq!(suggestions.len(), 0);
    }

    #[test]
    fn test_overlapping_suggestions() {
        let text = "I has a cat.";
        // "I has" (0..5) and "has" (2..5)
        // logic sorts by offset, then filters overlaps
        let matches = vec![
            LlmMatch {
                message: "long".to_string(),
                original: "I has".to_string(),
                replacement: Some("I have".to_string()),
                severity: Severity::Error,
            },
            LlmMatch {
                message: "short".to_string(),
                original: "has".to_string(),
                replacement: Some("have".to_string()),
                severity: Severity::Error,
            },
        ];

        let suggestions = convert_matches_to_suggestions(text, matches);
        // Should keep "I has" (starts at 0) and drop "has" (starts at 2, which is < 0+5)
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].original, "I has");
    }

    #[test]
    fn test_log_preview_is_unicode_safe() {
        let text = "é".repeat(150);
        let preview = log_preview(&text, 101);
        assert_eq!(preview.chars().count(), 101);
        assert!(text.starts_with(&preview));
    }
}
