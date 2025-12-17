use std::{net::SocketAddr, path::PathBuf};

use anyhow::Context;
use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use reqwest::header;
use serde::{Deserialize, Serialize};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    llm: LlmClient,
}

#[derive(Clone)]
struct LlmClient {
    http: reqwest::Client,
    api_base: String,
    model: String,
}

impl LlmClient {
    fn from_env() -> anyhow::Result<Self> {
        let api_key = std::env::var("GRAMMY_LLM_API_KEY")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| {
                std::env::var("OPENAI_API_KEY")
                    .ok()
                    .filter(|v| !v.trim().is_empty())
            })
            .context("OPENAI_API_KEY is required to enable the LLM")?;
        if api_key.trim().is_empty() {
            return Err(anyhow::anyhow!("OPENAI_API_KEY is required to enable the LLM"));
        }

        let api_base = std::env::var("GRAMMY_LLM_API_BASE")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let model = std::env::var("GRAMMY_LLM_MODEL")
            .unwrap_or_else(|_| "gpt-5-mini-2025-08-07".to_string());

        Self::new(api_base, api_key, model)
    }

    fn new(api_base: String, api_key: String, model: String) -> anyhow::Result<Self> {
        let mut headers = header::HeaderMap::new();
        let mut auth_value = header::HeaderValue::from_str(&format!("Bearer {}", api_key))
            .context("invalid API key")?;
        auth_value.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_value);

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self { http, api_base, model })
    }

    async fn check(&self, text: &str) -> anyhow::Result<Vec<Suggestion>> {
        let url = format!("{}/chat/completions", self.api_base.trim_end_matches('/'));

        let system = r#"You are a careful English writing assistant.
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
      "message": "...",
      "start": 0,
      "end": 0,
      "replacement": "..."
    }
  ]
}

Where start/end are CHARACTER indices (Unicode scalar value count) into the ORIGINAL input text. end is exclusive.
If there is nothing to change, return {"matches": []}.
"#;

        let user = format!("Text:\n{}", text);

        let body = OpenAiChatCompletionsRequest {
            model: self.model.clone(),
            temperature: None,
            messages: vec![
                OpenAiMessage {
                    role: "system".to_string(),
                    content: system.to_string(),
                },
                OpenAiMessage {
                    role: "user".to_string(),
                    content: user,
                },
            ],
            response_format: Some(OpenAiResponseFormat {
                r#type: "json_object".to_string(),
            }),
        };

        let res = self
            .http
            .post(url)
            .json(&body)
            .send()
            .await
            .context("LLM request failed")?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("LLM error {}: {}", status, text));
        }

        let payload: OpenAiChatCompletionsResponse = res.json().await.context("invalid LLM JSON")?;
        let content = payload
            .choices
            .get(0)
            .and_then(|c| c.message.content.clone())
            .unwrap_or_default();

        let parsed: LlmMatches = serde_json::from_str(&content).context("LLM returned non-JSON output")?;

        Ok(convert_llm_matches_to_suggestions(text, parsed.matches))
    }
}

#[derive(Debug, Serialize)]
struct OpenAiChatCompletionsRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<OpenAiResponseFormat>,
}

#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct OpenAiResponseFormat {
    r#type: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiChatCompletionsResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiAssistantMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiAssistantMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LlmMatches {
    matches: Vec<LlmMatch>,
}

#[derive(Debug, Deserialize)]
struct LlmMatch {
    message: String,
    start: usize,
    end: usize,
    replacement: String,
}

fn convert_llm_matches_to_suggestions(text: &str, matches: Vec<LlmMatch>) -> Vec<Suggestion> {
    let mut boundaries: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
    boundaries.push(text.len());
    let char_len = boundaries.len().saturating_sub(1);

    let mut out: Vec<Suggestion> = Vec::new();
    for m in matches {
        if m.start > m.end || m.end > char_len {
            continue;
        }

        let start_b = match boundaries.get(m.start) {
            Some(v) => *v,
            None => continue,
        };
        let end_b = match boundaries.get(m.end) {
            Some(v) => *v,
            None => continue,
        };

        if start_b > end_b || end_b > text.len() {
            continue;
        }

        let original = match text.get(start_b..end_b) {
            Some(v) => v,
            None => continue,
        };

        if original == m.replacement {
            continue;
        }

        out.push(Suggestion {
            id: Uuid::new_v4(),
            message: m.message,
            offset: start_b,
            length: end_b - start_b,
            original: original.to_string(),
            replacement: m.replacement,
            rule: "llm".to_string(),
        });
    }

    out.sort_by_key(|s| s.offset);

    let mut filtered: Vec<Suggestion> = Vec::with_capacity(out.len());
    let mut last_end = 0usize;
    for s in out {
        let end = s.offset.saturating_add(s.length);
        if s.offset < last_end {
            continue;
        }
        last_end = end;
        filtered.push(s);
    }

    filtered
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Suggestion {
    id: Uuid,
    message: String,
    offset: usize,
    length: usize,
    original: String,
    replacement: String,
    rule: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CheckRequest {
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CheckResponse {
    matches: Vec<Suggestion>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApplyRequest {
    text: String,
    suggestion: Suggestion,
}

#[derive(Debug, Serialize, Deserialize)]
struct ApplyResponse {
    text: String,
    matches: Vec<Suggestion>,
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let llm = match LlmClient::from_env() {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("{}", e);
            tracing::error!("Set OPENAI_API_KEY (or GRAMMY_LLM_API_KEY override) to run the server.");
            std::process::exit(1);
        }
    };

    let state = AppState { llm };

    let frontend_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..\\frontend");

    let static_service = ServeDir::new(&frontend_dir)
        .not_found_service(ServeFile::new(frontend_dir.join("index.html")));

    let app = Router::new()
        .route("/api/check", post(api_check))
        .route("/api/apply", post(api_apply))
        .fallback_service(static_service)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind listener");
    axum::serve(listener, app).await.expect("server error");
}

#[cfg(test)]
mod tests {
    use super::*;

    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, Request, Respond, ResponseTemplate,
    };

    struct BodyDoesNotContain(&'static str);

    impl wiremock::Match for BodyDoesNotContain {
        fn matches(&self, request: &Request) -> bool {
            let body = String::from_utf8_lossy(&request.body);
            !body.contains(self.0)
        }
    }

    struct BodyContains(&'static str);

    impl wiremock::Match for BodyContains {
        fn matches(&self, request: &Request) -> bool {
            let body = String::from_utf8_lossy(&request.body);
            body.contains(self.0)
        }
    }

    struct JsonResponder {
        body: serde_json::Value,
        status: u16,
    }

    impl Respond for JsonResponder {
        fn respond(&self, _request: &Request) -> ResponseTemplate {
            ResponseTemplate::new(self.status).set_body_json(self.body.clone())
        }
    }

    fn ok_chat_response(content_json: &str) -> serde_json::Value {
        serde_json::json!({
            "choices": [
                {
                    "message": {
                        "content": content_json
                    }
                }
            ]
        })
    }

    #[tokio::test]
    async fn llm_request_omits_temperature_and_uses_json_mode() {
        let server = MockServer::start().await;

        let responder = JsonResponder {
            status: 200,
            body: ok_chat_response(r#"{"matches": []}"#),
        };

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(BodyDoesNotContain("\"temperature\""))
            .and(BodyContains("\"response_format\""))
            .respond_with(responder)
            .mount(&server)
            .await;

        let client = LlmClient::new(server.uri(), "test-key".to_string(), "test-model".to_string())
            .expect("client");

        let res = client.check("Hello").await.expect("check ok");
        assert!(res.is_empty());
    }

    #[tokio::test]
    async fn llm_match_char_indices_convert_to_byte_offsets_unicode_safe() {
        let server = MockServer::start().await;

        let text = "Hi 😀 there";

        let content = r#"{"matches":[{"message":"Change","start":3,"end":4,"replacement":"🙂"}]}"#;
        let responder = JsonResponder {
            status: 200,
            body: ok_chat_response(content),
        };

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(responder)
            .mount(&server)
            .await;

        let client = LlmClient::new(server.uri(), "test-key".to_string(), "test-model".to_string())
            .expect("client");

        let res = client.check(text).await.expect("check ok");
        assert_eq!(res.len(), 1);
        let s = &res[0];

        assert_eq!(s.original, "😀");
        assert_eq!(s.replacement, "🙂");
        assert_eq!(&text[s.offset..s.offset + s.length], "😀");
    }

    #[tokio::test]
    async fn llm_non_success_status_is_returned_as_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(ResponseTemplate::new(400).set_body_string("bad request"))
            .mount(&server)
            .await;

        let client = LlmClient::new(server.uri(), "test-key".to_string(), "test-model".to_string())
            .expect("client");

        let err = client.check("Hello").await.expect_err("should error");
        let msg = err.to_string();
        assert!(msg.contains("400"));
    }

    #[tokio::test]
    #[ignore]
    async fn live_llm_smoke_test() {
        let run = std::env::var("GRAMMY_RUN_LIVE_TESTS").ok();
        if run.as_deref() != Some("1") {
            eprintln!("Skipping live test. Set GRAMMY_RUN_LIVE_TESTS=1 to enable.");
            return;
        }

        let api_key = std::env::var("GRAMMY_LLM_API_KEY")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or_else(|| {
                std::env::var("OPENAI_API_KEY")
                    .ok()
                    .filter(|v| !v.trim().is_empty())
            })
            .expect("OPENAI_API_KEY (or GRAMMY_LLM_API_KEY) must be set for live test");

        let api_base = std::env::var("GRAMMY_LLM_API_BASE")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let model = std::env::var("GRAMMY_LLM_MODEL")
            .unwrap_or_else(|_| "gpt-5-mini-2025-08-07".to_string());

        let client = LlmClient::new(api_base, api_key, model).expect("client");

        let text = "I am not totally fluent with english, but I want write better.";
        let matches = client.check(text).await.expect("LLM check should succeed");

        for s in matches {
            assert!(s.offset <= text.len());
            assert!(s.offset + s.length <= text.len());
            assert_eq!(&text[s.offset..s.offset + s.length], s.original);
        }
    }
}

async fn api_check(State(state): State<AppState>, Json(req): Json<CheckRequest>) -> impl IntoResponse {
    match state.llm.check(&req.text).await {
        Ok(matches) => (StatusCode::OK, Json(CheckResponse { matches })).into_response(),
        Err(e) => {
            tracing::warn!("LLM check failed: {}", e);
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    error: format!("LLM check failed: {}", e),
                }),
            )
                .into_response()
        }
    }
}

async fn api_apply(
    State(_state): State<AppState>,
    Json(req): Json<ApplyRequest>,
) -> impl IntoResponse {
    let s = req.suggestion;

    let start = s.offset;
    let end = s.offset.saturating_add(s.length);

    if start > req.text.len() || end > req.text.len() || start > end {
        return (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid suggestion range".to_string(),
            }),
        )
            .into_response();
    }

    let slice = &req.text[start..end];
    if slice != s.original {
        return (
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Text changed since the last check; please run Check again".to_string(),
            }),
        )
            .into_response();
    }

    let mut new_text = String::with_capacity(req.text.len() + s.replacement.len());
    new_text.push_str(&req.text[..start]);
    new_text.push_str(&s.replacement);
    new_text.push_str(&req.text[end..]);

    // Return empty matches - frontend will handle offset adjustment for remaining suggestions
    // This makes apply instant instead of waiting for another LLM call
    (StatusCode::OK, Json(ApplyResponse { text: new_text, matches: vec![] })).into_response()
}
