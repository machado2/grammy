use crate::config::ApiProvider;
use crate::suggestion::{LlmMatch, LlmResponse, Suggestion};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

const SYSTEM_PROMPT: &str = r#"You are a careful English writing assistant.
Your job: suggest minimal edits for grammar, clarity, and phrases that sound non-native/awkward.
Rules:
- Do NOT rewrite the whole text.
- Only propose small localized edits (replace a short span with a short span).
- Preserve the author's voice and meaning.
- Prefer fewer suggestions over many.

Return ONLY valid JSON with this exact schema:
{
  "matches": [
    {
      "message": "short explanation",
      "original": "exact text to replace",
      "replacement": "corrected text"
    }
  ]
}

IMPORTANT: The "original" field must contain the EXACT substring from the input that should be replaced (copy it precisely, including spacing).
If there is nothing to change, return {"matches": []}."#;

pub async fn check_grammar(
    text: String,
    api_key: String,
    model: String,
    provider: ApiProvider,
    request_id: u64,
) -> Result<(Vec<Suggestion>, u64), String> {
    let start = Instant::now();
    eprintln!("[DEBUG #{request_id}] Starting grammar check, provider={}, model={}, text_len={}", 
              provider.name(), model, text.len());

    if api_key.is_empty() {
        eprintln!("[DEBUG #{request_id}] Error: API key not set");
        return Err("API key not set. Click ⚙ to configure.".to_string());
    }

    if text.trim().is_empty() {
        eprintln!("[DEBUG #{request_id}] Empty text, returning no suggestions");
        return Ok((vec![], request_id));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    let body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": format!("Text:\n{}", text) }
        ],
        "response_format": { "type": "json_object" }
    });

    // OpenRouter requires additional headers and may need different body format
    let url = provider.base_url();
    
    eprintln!("[DEBUG #{request_id}] Sending request to {}", url);

    let mut request = client
        .post(url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key));

    // Add OpenRouter-specific headers
    if provider == ApiProvider::OpenRouter {
        request = request
            .header("HTTP-Referer", "https://github.com/grammy-app")
            .header("X-Title", "Grammy");
    }

    let response = request
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            eprintln!("[DEBUG #{request_id}] Network error after {:?}: {}", start.elapsed(), e);
            format!("Network error: {}", e)
        })?;

    let status = response.status();
    eprintln!("[DEBUG #{request_id}] Response status: {} after {:?}", status, start.elapsed());

    if !status.is_success() {
        let error_body: serde_json::Value = response.json().await.unwrap_or_default();
        let msg = error_body["error"]["message"]
            .as_str()
            .unwrap_or("Unknown error");
        eprintln!("[DEBUG #{request_id}] API error: {} - {}", status, msg);
        return Err(format!("{} error ({}): {}", provider.name(), status, msg));
    }

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|e| {
            eprintln!("[DEBUG #{request_id}] Failed to parse response: {}", e);
            format!("Failed to parse response: {}", e)
        })?;

    let content = data["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or(r#"{"matches":[]}"#);

    eprintln!("[DEBUG #{request_id}] LLM response content: {}", &content[..content.len().min(200)]);

    let llm_response: LlmResponse = serde_json::from_str(content).map_err(|e| {
        eprintln!("[DEBUG #{request_id}] Invalid JSON from LLM: {}", e);
        format!("Invalid JSON from LLM: {}", e)
    })?;

    let suggestions = convert_matches_to_suggestions(&text, llm_response.matches);
    eprintln!("[DEBUG #{request_id}] Completed in {:?}, found {} suggestions", start.elapsed(), suggestions.len());

    Ok((suggestions, request_id))
}

pub fn next_request_id() -> u64 {
    REQUEST_COUNTER.fetch_add(1, Ordering::SeqCst)
}

fn convert_matches_to_suggestions(text: &str, matches: Vec<LlmMatch>) -> Vec<Suggestion> {
    let mut suggestions = Vec::new();

    for m in matches {
        if m.original.is_empty() || m.replacement.is_empty() {
            continue;
        }
        if m.original == m.replacement {
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
