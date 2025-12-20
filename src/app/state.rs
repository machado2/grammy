use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

use iced::widget::text_editor;
use iced::{window, Subscription, Task, Theme};

use crate::config::{ApiProvider, Config};
use crate::suggestion::Suggestion;

use super::api_worker::{spawn_api_worker, ApiJob, ApiRequest, ApiResponse};
use super::draft;
use super::history::MessageHistory;
use super::style;
use super::ui;

// DEBOUNCE_MS removed, using config instead
const TICK_MS: u64 = 50;
const AUTOSAVE_SECS: u64 = 30;

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    AutosaveTick,
    WindowCloseRequested(window::Id),

    EditorAction(text_editor::Action),
    ApplySuggestion(String),
    DismissSuggestion(String),
    HoverSuggestion(String),
    ClearHoverSuggestion,

    ForceCheck,

    OpenSettings,
    CloseSettings,
    ToggleShowApiKey,

    SelectProvider(ApiProvider),
    TempOpenAiKeyChanged(String),
    TempOpenRouterKeyChanged(String),
    TempGeminiKeyChanged(String),
    TempModelChanged(String),
    TempDebounceChanged(f32),
    ModelSelected(String),

    SaveSettings,
    StartTestConnection,
}

pub struct State {
    pub(super) editor: text_editor::Content,
    pub(super) last_checked_text: String,
    pub(super) suggestions: Vec<Suggestion>,

    pub(super) draft_dirty: bool,

    pub(super) hovered_suggestion: Option<String>,

    pub(super) status: String,

    pub(super) config: Config,

    pub(super) show_settings: bool,
    pub(super) show_api_key: bool,
    pub(super) temp_openai_api_key: String,
    pub(super) temp_openrouter_api_key: String,
    pub(super) temp_gemini_api_key: String,
    pub(super) temp_model: String,
    pub(super) temp_provider: ApiProvider,
    pub(super) temp_debounce_ms: f32,

    pub(super) openai_models: Vec<String>,
    pub(super) openrouter_models: Vec<String>,
    pub(super) gemini_models: Vec<String>,
    pub(super) model_combo_state: iced::widget::combo_box::State<String>,

    pub(super) test_status: String,
    pub(super) is_testing: bool,
    pub(super) current_test_request_id: Option<u64>,

    pub(super) last_edit_time: Option<Instant>,
    pub(super) is_checking: bool,
    pub(super) current_check_request_id: Option<u64>,
    pub(super) pending_recheck: bool,
    pub(super) pending_check_text: Option<String>,

    pub(super) message_history: MessageHistory,

    pub(super) api_sender: Sender<ApiRequest>,
    pub(super) api_receiver: Receiver<ApiResponse>,
}

pub fn new() -> (State, Task<Message>) {
    let config = Config::load();

    let (request_tx, request_rx) = channel::<ApiRequest>();
    let (response_tx, response_rx) = channel::<ApiResponse>();
    spawn_api_worker(request_rx, response_tx);

    let draft = draft::load();
    let editor = if draft.text.is_empty() {
        text_editor::Content::new()
    } else {
        text_editor::Content::with_text(&draft.text)
    };

    (
        State {
            editor,
            last_checked_text: String::new(),
            suggestions: Vec::new(),

            draft_dirty: false,

            hovered_suggestion: None,
            status: "Ready".to_string(),
            config: config.clone(),
            show_settings: false,
            show_api_key: false,
            temp_openai_api_key: config.openai_api_key.clone(),
            temp_openrouter_api_key: config.openrouter_api_key.clone(),
            temp_gemini_api_key: config.gemini_api_key.clone(),
            temp_model: config.model,
            temp_provider: config.provider,
            temp_debounce_ms: config.debounce_ms as f32,

            openai_models: Vec::new(),
            openrouter_models: Vec::new(),
            gemini_models: Vec::new(),
            model_combo_state: iced::widget::combo_box::State::new(Vec::new()),

            test_status: String::new(),
            is_testing: false,
            current_test_request_id: None,
            last_edit_time: None,
            is_checking: false,
            current_check_request_id: None,
            pending_recheck: false,
            pending_check_text: None,
            message_history: MessageHistory::default(),
            api_sender: request_tx,
            api_receiver: response_rx,
        },
        Task::none(),
    )
}

pub fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::Tick => {
            process_api_responses(state);
            tick_debounce(state);
            Task::none()
        }

        Message::AutosaveTick => {
            if state.draft_dirty {
                draft::save_text(state.editor.text());
                state.draft_dirty = false;
            }
            Task::none()
        }

        Message::WindowCloseRequested(id) => {
            if state.draft_dirty {
                draft::save_text(state.editor.text());
                state.draft_dirty = false;
            }
            window::close(id)
        }

        Message::EditorAction(action) => {
            let old_text = state.editor.text();
            state.editor.perform(action);
            let new_text = state.editor.text();

            // Only clear suggestions if text actually changed
            if old_text != new_text {
                state.suggestions.clear();
                state.hovered_suggestion = None;
                state.last_edit_time = Some(Instant::now());
                state.draft_dirty = true;
                if state.is_checking {
                    state.pending_recheck = true;
                }
            }
            Task::none()
        }

        Message::ApplySuggestion(id) => {
            let old_text = state.editor.text();
            apply_suggestion(state, &id);
            if state.editor.text() != old_text {
                state.draft_dirty = true;
            }
            Task::none()
        }

        Message::DismissSuggestion(id) => {
            state.suggestions.retain(|s| s.id != id);
            if state.hovered_suggestion.as_deref() == Some(id.as_str()) {
                state.hovered_suggestion = None;
            }

            if !state.is_checking {
                if state.suggestions.is_empty() {
                    state.status = "All good!".to_string();
                } else {
                    state.status = format!("{} suggestion(s)", state.suggestions.len());
                }
            }

            Task::none()
        }

        Message::HoverSuggestion(id) => {
            state.hovered_suggestion = Some(id);
            Task::none()
        }

        Message::ClearHoverSuggestion => {
            state.hovered_suggestion = None;
            Task::none()
        }

        Message::ForceCheck => {
            if state.is_checking {
                state.pending_recheck = true;
                state.status = "Rechecking...".to_string();
                return Task::none();
            }

            state.last_checked_text.clear();
            check_text(state);
            Task::none()
        }

        Message::OpenSettings => {
            state.temp_openai_api_key = state.config.openai_api_key.clone();
            state.temp_openrouter_api_key = state.config.openrouter_api_key.clone();
            state.temp_gemini_api_key = state.config.gemini_api_key.clone();
            state.temp_model = state.config.model.clone();
            state.temp_provider = state.config.provider.clone();
            state.temp_debounce_ms = state.config.debounce_ms as f32;
            state.show_api_key = false;
            state.test_status.clear();
            state.show_settings = true;

            // Trigger model fetching for current provider
            fetch_models_if_needed(state);
            Task::none()
        }
        Message::CloseSettings => {
            state.show_settings = false;
            Task::none()
        }
        Message::ToggleShowApiKey => {
            state.show_api_key = !state.show_api_key;
            Task::none()
        }

        Message::SelectProvider(p) => {
            state.temp_provider = p.clone();
            state.temp_model = state.temp_provider.default_model().to_string();
            state.test_status.clear();
            state.show_api_key = false;
            fetch_models_if_needed(state);
            Task::none()
        }

        Message::TempOpenAiKeyChanged(v) => {
            state.temp_openai_api_key = v;
            fetch_models_if_needed(state);
            Task::none()
        }
        Message::TempOpenRouterKeyChanged(v) => {
            state.temp_openrouter_api_key = v;
            fetch_models_if_needed(state);
            Task::none()
        }
        Message::TempGeminiKeyChanged(v) => {
            state.temp_gemini_api_key = v;
            fetch_models_if_needed(state);
            Task::none()
        }
        Message::TempModelChanged(v) => {
            state.temp_model = v;
            Task::none()
        }
        Message::TempDebounceChanged(v) => {
            state.temp_debounce_ms = v;
            Task::none()
        }
        Message::ModelSelected(v) => {
            state.temp_model = v;
            Task::none()
        }

        Message::SaveSettings => {
            state.config.openai_api_key = state.temp_openai_api_key.trim().to_string();
            state.config.openrouter_api_key = state.temp_openrouter_api_key.trim().to_string();
            state.config.gemini_api_key = state.temp_gemini_api_key.trim().to_string();
            state.config.provider = state.temp_provider.clone();
            state.config.model = if state.temp_model.trim().is_empty() {
                state.config.provider.default_model().to_string()
            } else {
                state.temp_model.trim().to_string()
            };
            state.config.debounce_ms = state.temp_debounce_ms as u64;
            state.config.save();
            state.show_settings = false;
            state.status = "Settings saved".to_string();
            Task::none()
        }

        Message::StartTestConnection => {
            if state.is_testing {
                return Task::none();
            }

            let request_id = crate::api::next_request_id();
            state.is_testing = true;
            state.current_test_request_id = Some(request_id);
            state.test_status = "Testing...".to_string();

            let api_key = match state.temp_provider {
                ApiProvider::OpenAI => state.temp_openai_api_key.trim().to_string(),
                ApiProvider::OpenRouter => state.temp_openrouter_api_key.trim().to_string(),
                ApiProvider::Gemini => state.temp_gemini_api_key.trim().to_string(),
            };

            let request = ApiRequest {
                job: ApiJob::TestConnection {
                    api_key,
                    provider: state.temp_provider.clone(),
                    model: state.temp_model.clone(),
                },
                request_id,
            };

            if let Err(e) = state.api_sender.send(request) {
                state.is_testing = false;
                state.current_test_request_id = None;
                state.test_status = format!("Internal error: failed to send test ({})", e);
            }

            Task::none()
        }
    }
}

pub fn view(state: &State) -> iced::Element<'_, Message> {
    ui::view(state)
}

pub fn theme(state: &State) -> Theme {
    style::theme(state)
}

pub fn subscription(_state: &State) -> Subscription<Message> {
    Subscription::batch([
        iced::time::every(Duration::from_millis(TICK_MS)).map(|_| Message::Tick),
        iced::time::every(Duration::from_secs(AUTOSAVE_SECS)).map(|_| Message::AutosaveTick),
        window::close_requests().map(Message::WindowCloseRequested),
    ])
}

pub fn settings() -> iced::Settings {
    iced::Settings {
        default_text_size: 14.0.into(),
        ..Default::default()
    }
}

fn tick_debounce(state: &mut State) {
    if state.last_edit_time.is_none() {
        return;
    }

    if let Some(edit_time) = state.last_edit_time {
        let delay = state.config.debounce_ms;
        // If delay > 5000, we treat it as "Never"
        if delay > 5000 {
            return;
        }
        if edit_time.elapsed() >= Duration::from_millis(delay) {
            state.last_edit_time = None;
            check_text(state);
        }
    }
}

fn check_text(state: &mut State) {
    let text = state.editor.text();

    if text.trim().is_empty() {
        state.suggestions.clear();
        state.hovered_suggestion = None;
        state.status = "Ready".to_string();
        state.last_checked_text = text;
        state.is_checking = false;
        state.current_check_request_id = None;
        return;
    }

    if text == state.last_checked_text {
        return;
    }

    if state.is_checking {
        state.pending_recheck = true;
        return;
    }

    let request_id = crate::api::next_request_id();

    state.is_checking = true;
    state.current_check_request_id = Some(request_id);
    state.status = "Checking...".to_string();

    state.suggestions.clear();
    state.hovered_suggestion = None;
    state.last_checked_text = text.clone();

    let request = ApiRequest {
        job: ApiJob::Grammar {
            text: text.clone(),
            api_key: state.config.api_key_for_provider(&state.config.provider),
            model: state.config.model.clone(),
            provider: state.config.provider.clone(),
            history: state
                .message_history
                .get_entries()
                .into_iter()
                .cloned()
                .collect(),
        },
        request_id,
    };

    // Store the text for later use in history
    state.pending_check_text = Some(text);

    if let Err(e) = state.api_sender.send(request) {
        state.status = format!("Internal error: failed to send request ({})", e);
        state.is_checking = false;
        state.current_check_request_id = None;
    }
}

fn process_api_responses(state: &mut State) {
    loop {
        match state.api_receiver.try_recv() {
            Ok(response) => match response {
                ApiResponse::GrammarSuccess {
                    suggestions,
                    request_id,
                } => {
                    if state.current_check_request_id != Some(request_id) {
                        continue;
                    }

                    state.is_checking = false;
                    state.current_check_request_id = None;

                    // Save to history for cycle prevention
                    if let Some(user_text) = state.pending_check_text.take() {
                        // Format LLM response as JSON for history context
                        let assistant_content = if suggestions.is_empty() {
                            r#"{"matches":[]}"#.to_string()
                        } else {
                            serde_json::to_string(&serde_json::json!({
                                "matches": suggestions.iter().map(|s| {
                                    serde_json::json!({
                                        "message": s.message,
                                        "original": s.original,
                                        "replacement": s.replacement,
                                        "severity": format!("{:?}", s.severity).to_lowercase()
                                    })
                                }).collect::<Vec<_>>()
                            }))
                            .unwrap_or_else(|_| r#"{"matches":[]}"#.to_string())
                        };
                        state
                            .message_history
                            .push_pair(format!("Text:\n{}", user_text), assistant_content);
                    }

                    state.suggestions = suggestions;
                    if state.suggestions.is_empty() {
                        state.status = "All good!".to_string();
                    } else {
                        state.status = format!("{} suggestion(s)", state.suggestions.len());
                    }

                    if state.pending_recheck {
                        let delay = state.config.debounce_ms;
                        if delay <= 5000 {
                            state.last_edit_time =
                                Some(Instant::now() - Duration::from_millis(delay));
                        } else {
                            state.pending_recheck = false;
                        }
                    }
                }
                ApiResponse::GrammarError {
                    message,
                    request_id,
                } => {
                    if state.current_check_request_id != Some(request_id) {
                        continue;
                    }

                    state.is_checking = false;
                    state.current_check_request_id = None;
                    state.status = message;

                    if state.pending_recheck {
                        let delay = state.config.debounce_ms;
                        // Only recheck if auto-check is enabled (<= 5000)
                        if delay <= 5000 {
                            state.last_edit_time =
                                Some(Instant::now() - Duration::from_millis(delay));
                        } else {
                            state.pending_recheck = false; // Cancel pending recheck if disabled
                        }
                    }
                }
                ApiResponse::TestSuccess { request_id } => {
                    if state.current_test_request_id != Some(request_id) {
                        continue;
                    }

                    state.is_testing = false;
                    state.current_test_request_id = None;
                    state.test_status = "Connection OK".to_string();
                }
                ApiResponse::TestError {
                    message,
                    request_id,
                } => {
                    if state.current_test_request_id != Some(request_id) {
                        continue;
                    }

                    state.is_testing = false;
                    state.current_test_request_id = None;
                    state.test_status = message;
                }
                ApiResponse::ModelsSuccess { models, provider } => {
                    match provider {
                        ApiProvider::OpenAI => state.openai_models = models,
                        ApiProvider::OpenRouter => state.openrouter_models = models,
                        ApiProvider::Gemini => state.gemini_models = models,
                    }
                    if provider == state.temp_provider {
                        let models = match state.temp_provider {
                            ApiProvider::OpenAI => &state.openai_models,
                            ApiProvider::OpenRouter => &state.openrouter_models,
                            ApiProvider::Gemini => &state.gemini_models,
                        };
                        state.model_combo_state =
                            iced::widget::combo_box::State::new(models.clone());
                    }
                }
                ApiResponse::ModelsError { message } => {
                    eprintln!("[DEBUG] Failed to fetch models: {}", message);
                }
            },
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                state.status = "Internal error: API thread died".to_string();
                break;
            }
        }
    }
}

fn fetch_models_if_needed(state: &mut State) {
    let api_key = match state.temp_provider {
        ApiProvider::OpenAI => &state.temp_openai_api_key,
        ApiProvider::OpenRouter => &state.temp_openrouter_api_key,
        ApiProvider::Gemini => &state.temp_gemini_api_key,
    };

    if api_key.is_empty() {
        return;
    }

    // Check if we already have models for this provider
    let has_models = match state.temp_provider {
        ApiProvider::OpenAI => !state.openai_models.is_empty(),
        ApiProvider::OpenRouter => !state.openrouter_models.is_empty(),
        ApiProvider::Gemini => !state.gemini_models.is_empty(),
    };

    if has_models {
        let models = match state.temp_provider {
            ApiProvider::OpenAI => &state.openai_models,
            ApiProvider::OpenRouter => &state.openrouter_models,
            ApiProvider::Gemini => &state.gemini_models,
        };
        state.model_combo_state = iced::widget::combo_box::State::new(models.clone());
    }

    let request_id = crate::api::next_request_id();
    let request = ApiRequest {
        job: ApiJob::FetchModels {
            api_key: api_key.clone(),
            provider: state.temp_provider.clone(),
        },
        request_id,
    };

    let _ = state.api_sender.send(request);
}

fn apply_suggestion(state: &mut State, suggestion_id: &str) {
    let suggestion = state
        .suggestions
        .iter()
        .find(|s| s.id == suggestion_id)
        .cloned();

    let Some(suggestion) = suggestion else {
        return;
    };

    let text = state.editor.text();
    let start = suggestion.offset;
    let end = suggestion.offset + suggestion.length;

    if start > text.len() || end > text.len() {
        state.status = "Invalid suggestion range".to_string();
        state.last_edit_time = Some(Instant::now());
        return;
    }

    let slice = &text[start..end];
    if slice != suggestion.original {
        state.status = "Text changed; re-checking...".to_string();
        state.last_edit_time = Some(Instant::now());
        return;
    }

    let replacement = match &suggestion.replacement {
        Some(r) => r,
        None => return, // Cannot apply a comment-only suggestion
    };

    let new_text = format!("{}{}{}", &text[..start], replacement, &text[end..]);

    let delta = replacement.len() as isize - suggestion.length as isize;

    state.suggestions.retain(|s| s.id != suggestion_id);
    for s in &mut state.suggestions {
        if s.offset > suggestion.offset {
            s.offset = (s.offset as isize + delta) as usize;
        }
    }

    state.editor = text_editor::Content::with_text(&new_text);
    state.last_checked_text = new_text;

    if state.suggestions.is_empty() {
        state.status = "All good!".to_string();
    } else {
        state.status = format!("{} suggestion(s)", state.suggestions.len());
    }
}
