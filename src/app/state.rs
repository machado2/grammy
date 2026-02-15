use std::sync::{
    mpsc::{channel, Receiver, Sender, TryRecvError},
    Arc,
};
use std::time::{Duration, Instant};

use iced::widget::text_editor;
use iced::{window, Subscription, Task, Theme};

use crate::config::{ApiProvider, Config, ReasoningEffort};
use crate::suggestion::Suggestion;

use super::api_worker::{spawn_api_worker, ApiJob, ApiRequest, ApiResponse};
use super::draft;
use super::highlight;
use super::history::MessageHistory;
use super::style;
use super::ui;

// DEBOUNCE_MS removed, using config instead
const TICK_MS: u64 = 50;
const AUTOSAVE_SECS: u64 = 30;
const MODELS_FETCH_DEBOUNCE_MS: u64 = 400;
const HISTORY_MAX_CHARS: usize = 8_000;
const HISTORY_MAX_MATCHES: usize = 20;
const HISTORY_MAX_EXCERPTS: usize = 8;
const HISTORY_FIELD_MAX_CHARS: usize = 200;
const HISTORY_EXCERPT_MAX_CHARS: usize = 220;
const HISTORY_NO_SUGGESTIONS_MAX_CHARS: usize = 800;

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    AutosaveTick,
    WindowCloseRequested(window::Id),

    EditorAction(text_editor::Action),
    Undo,
    Redo,
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
    SelectReasoningEffort(ReasoningEffort),
    ModelSelected(String),

    SaveSettings,
    StartTestConnection,
}

pub struct State {
    pub(super) editor: text_editor::Content,
    pub(super) editor_text: String,
    pub(super) editor_revision: u64,
    pub(super) highlight_line_starts: Arc<Vec<usize>>,
    pub(super) highlight_suggestion_spans: Arc<Vec<highlight::Span>>,
    pub(super) highlight_code_spans: Arc<Vec<highlight::Span>>,
    pub(super) highlight_settings: highlight::Settings,
    pub(super) last_checked_revision: Option<u64>,
    pub(super) suggestions: Vec<Suggestion>,
    pub(super) undo_stack: Vec<EditorSnapshot>,
    pub(super) redo_stack: Vec<EditorSnapshot>,

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
    pub(super) temp_reasoning_effort: ReasoningEffort,

    pub(super) openai_models: Vec<String>,
    pub(super) openrouter_models: Vec<String>,
    pub(super) gemini_models: Vec<String>,
    pub(super) model_combo_state: iced::widget::combo_box::State<String>,

    pub(super) models_fetch_debounce_start: Option<Instant>,
    pub(super) openai_models_loaded_for_key: Option<String>,
    pub(super) openrouter_models_loaded_for_key: Option<String>,
    pub(super) gemini_models_loaded_for_key: Option<String>,
    pub(super) openai_models_request_id: Option<u64>,
    pub(super) openrouter_models_request_id: Option<u64>,
    pub(super) gemini_models_request_id: Option<u64>,

    pub(super) test_status: String,
    pub(super) is_testing: bool,
    pub(super) current_test_request_id: Option<u64>,

    pub(super) last_edit_time: Option<Instant>,
    pub(super) is_checking: bool,
    pub(super) current_check_request_id: Option<u64>,
    pub(super) current_check_revision: Option<u64>,
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
    let editor_text = draft.text.clone();
    let editor = if draft.text.is_empty() {
        text_editor::Content::new()
    } else {
        text_editor::Content::with_text(&draft.text)
    };

    let highlight_line_starts = Arc::new(highlight::compute_line_starts(&editor_text));
    let highlight_code_spans = Arc::new(highlight::spans_from_backticks(&editor_text));
    let highlight_suggestion_spans = Arc::new(Vec::new());
    let highlight_settings = highlight::Settings {
        line_starts: highlight_line_starts.clone(),
        suggestion_spans: highlight_suggestion_spans.clone(),
        code_spans: highlight_code_spans.clone(),
    };

    (
        State {
            editor,
            editor_text,
            editor_revision: 0,
            highlight_line_starts,
            highlight_suggestion_spans,
            highlight_code_spans,
            highlight_settings,
            last_checked_revision: None,
            suggestions: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),

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
            temp_reasoning_effort: config.reasoning_effort,

            openai_models: Vec::new(),
            openrouter_models: Vec::new(),
            gemini_models: Vec::new(),
            model_combo_state: iced::widget::combo_box::State::new(Vec::new()),

            models_fetch_debounce_start: None,
            openai_models_loaded_for_key: None,
            openrouter_models_loaded_for_key: None,
            gemini_models_loaded_for_key: None,
            openai_models_request_id: None,
            openrouter_models_request_id: None,
            gemini_models_request_id: None,

            test_status: String::new(),
            is_testing: false,
            current_test_request_id: None,
            last_edit_time: None,
            is_checking: false,
            current_check_request_id: None,
            current_check_revision: None,
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
            tick_models_fetch_debounce(state);
            Task::none()
        }

        Message::AutosaveTick => {
            if state.draft_dirty {
                draft::save_text(state.editor_text.clone());
                state.draft_dirty = false;
            }
            Task::none()
        }

        Message::WindowCloseRequested(id) => {
            if state.draft_dirty {
                draft::save_text(state.editor_text.clone());
                state.draft_dirty = false;
            }
            window::close(id)
        }

        Message::EditorAction(action) => {
            let old_text = state.editor_text.clone();
            let old_cursor = state.editor.cursor();
            state.editor.perform(action);
            let new_text = state.editor.text();

            // Only clear suggestions if text actually changed
            if old_text != new_text {
                let old_revision = state.editor_revision;
                state.undo_stack.push(EditorSnapshot {
                    text: old_text,
                    cursor: old_cursor,
                });
                state.redo_stack.clear();
                state.suggestions.clear();
                state.hovered_suggestion = None;
                state.editor_text = new_text;
                state.editor_revision = state.editor_revision.wrapping_add(1);
                eprintln!(
                    "[DEBUG] Editor text changed (revision {} -> {}, text_len={})",
                    old_revision,
                    state.editor_revision,
                    state.editor_text.len()
                );
                rebuild_text_highlight_cache(state);
                rebuild_suggestion_highlight_cache(state);
                refresh_highlight_settings(state);
                state.last_edit_time = Some(Instant::now());
                state.draft_dirty = true;
                if state.is_checking {
                    eprintln!(
                        "[DEBUG] Editor change detected during check, cancelling active request"
                    );
                    cancel_inflight_grammar_check(state);
                }
            }
            Task::none()
        }

        Message::Undo => {
            let current_snapshot = EditorSnapshot {
                text: state.editor_text.clone(),
                cursor: state.editor.cursor(),
            };
            let old_text = current_snapshot.text.clone();

            if let Some(previous) = state.undo_stack.pop() {
                state.redo_stack.push(current_snapshot);
                restore_snapshot(state, previous);
                if state.editor_text != old_text {
                    state.suggestions.clear();
                    state.hovered_suggestion = None;
                    rebuild_suggestion_highlight_cache(state);
                    refresh_highlight_settings(state);
                    state.last_edit_time = Some(Instant::now());
                    state.draft_dirty = true;
                    if state.is_checking {
                        cancel_inflight_grammar_check(state);
                    }
                }
            }
            Task::none()
        }

        Message::Redo => {
            let current_snapshot = EditorSnapshot {
                text: state.editor_text.clone(),
                cursor: state.editor.cursor(),
            };
            let old_text = current_snapshot.text.clone();

            if let Some(next) = state.redo_stack.pop() {
                state.undo_stack.push(current_snapshot);
                restore_snapshot(state, next);
                if state.editor_text != old_text {
                    state.suggestions.clear();
                    state.hovered_suggestion = None;
                    rebuild_suggestion_highlight_cache(state);
                    refresh_highlight_settings(state);
                    state.last_edit_time = Some(Instant::now());
                    state.draft_dirty = true;
                    if state.is_checking {
                        cancel_inflight_grammar_check(state);
                    }
                }
            }
            Task::none()
        }

        Message::ApplySuggestion(id) => {
            let old_text = state.editor_text.clone();
            let old_cursor = state.editor.cursor();
            apply_suggestion(state, &id);
            if state.editor_text != old_text {
                state.undo_stack.push(EditorSnapshot {
                    text: old_text,
                    cursor: old_cursor,
                });
                state.redo_stack.clear();
                state.draft_dirty = true;
            }
            Task::none()
        }

        Message::DismissSuggestion(id) => {
            state.suggestions.retain(|s| s.id != id);
            if state.hovered_suggestion.as_deref() == Some(id.as_str()) {
                state.hovered_suggestion = None;
            }
            rebuild_suggestion_highlight_cache(state);
            refresh_highlight_settings(state);

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
            rebuild_suggestion_highlight_cache(state);
            refresh_highlight_settings(state);
            Task::none()
        }

        Message::ClearHoverSuggestion => {
            state.hovered_suggestion = None;
            rebuild_suggestion_highlight_cache(state);
            refresh_highlight_settings(state);
            Task::none()
        }

        Message::ForceCheck => {
            eprintln!(
                "[DEBUG] ForceCheck requested at revision {} (previous last_checked_rev={:?})",
                state.editor_revision, state.last_checked_revision
            );
            state.last_checked_revision = None;
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
            state.temp_reasoning_effort = state.config.reasoning_effort;
            state.show_api_key = false;
            state.test_status.clear();
            state.show_settings = true;
            state.models_fetch_debounce_start = None;

            // Trigger model fetching for current provider
            fetch_models_if_needed(state);
            Task::none()
        }
        Message::CloseSettings => {
            state.show_settings = false;
            state.models_fetch_debounce_start = None;
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
            state.models_fetch_debounce_start = None;
            fetch_models_if_needed(state);
            Task::none()
        }

        Message::TempOpenAiKeyChanged(v) => {
            state.temp_openai_api_key = v;
            invalidate_models_cache_for_provider(state, ApiProvider::OpenAI);
            schedule_models_fetch(state);
            Task::none()
        }
        Message::TempOpenRouterKeyChanged(v) => {
            state.temp_openrouter_api_key = v;
            invalidate_models_cache_for_provider(state, ApiProvider::OpenRouter);
            schedule_models_fetch(state);
            Task::none()
        }
        Message::TempGeminiKeyChanged(v) => {
            state.temp_gemini_api_key = v;
            invalidate_models_cache_for_provider(state, ApiProvider::Gemini);
            schedule_models_fetch(state);
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
        Message::SelectReasoningEffort(v) => {
            state.temp_reasoning_effort = v;
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
            state.config.reasoning_effort = state.temp_reasoning_effort;
            state.config.save();
            state.show_settings = false;
            state.models_fetch_debounce_start = None;
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

pub fn subscription(state: &State) -> Subscription<Message> {
    let needs_tick = state.last_edit_time.is_some()
        || state.is_checking
        || state.is_testing
        || state.show_settings;

    let mut subs = Vec::new();

    if needs_tick {
        subs.push(iced::time::every(Duration::from_millis(TICK_MS)).map(|_| Message::Tick));
    }

    subs.push(iced::time::every(Duration::from_secs(AUTOSAVE_SECS)).map(|_| Message::AutosaveTick));
    subs.push(window::close_requests().map(Message::WindowCloseRequested));

    Subscription::batch(subs)
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
            eprintln!(
                "[DEBUG] Debounce fired at revision {} after {:?} (delay={}ms)",
                state.editor_revision,
                edit_time.elapsed(),
                delay
            );
            state.last_edit_time = None;
            check_text(state);
        }
    }
}

fn log_grammar_state(state: &State, context: &str) {
    eprintln!(
        "[DEBUG] {} | checking={} current_req={:?} current_req_rev={:?} editor_rev={} last_checked_rev={:?} pending_text_len={}",
        context,
        state.is_checking,
        state.current_check_request_id,
        state.current_check_revision,
        state.editor_revision,
        state.last_checked_revision,
        state.pending_check_text.as_ref().map(|t| t.len()).unwrap_or(0)
    );
}

fn check_text(state: &mut State) {
    let text = state.editor_text.clone();
    eprintln!(
        "[DEBUG] check_text called (editor_rev={}, last_checked_rev={:?}, text_len={})",
        state.editor_revision,
        state.last_checked_revision,
        text.len()
    );
    log_grammar_state(state, "Before check_text decision");

    if text.trim().is_empty() {
        eprintln!("[DEBUG] check_text: text is empty, cancelling in-flight request");
        cancel_inflight_grammar_check(state);
        state.suggestions.clear();
        state.hovered_suggestion = None;
        rebuild_suggestion_highlight_cache(state);
        refresh_highlight_settings(state);
        state.status = "Ready".to_string();
        state.last_checked_revision = Some(state.editor_revision);
        return;
    }

    if state.last_checked_revision == Some(state.editor_revision) {
        eprintln!(
            "[DEBUG] check_text: skipping request, revision {} already checked",
            state.editor_revision
        );
        return;
    }

    let request_id = crate::api::next_request_id();

    state.is_checking = true;
    state.current_check_request_id = Some(request_id);
    state.current_check_revision = Some(state.editor_revision);
    state.status = "Checking...".to_string();

    state.suggestions.clear();
    state.hovered_suggestion = None;
    rebuild_suggestion_highlight_cache(state);
    refresh_highlight_settings(state);
    state.last_checked_revision = Some(state.editor_revision);

    let request = ApiRequest {
        job: ApiJob::Grammar {
            text: text.clone(),
            api_key: state.config.api_key_for_provider(&state.config.provider),
            model: state.config.model.clone(),
            provider: state.config.provider.clone(),
            reasoning_effort: state.config.reasoning_effort,
            history: state
                .message_history
                .get_entries_limited_pairs(HISTORY_MAX_CHARS)
                .into_iter()
                .cloned()
                .collect(),
        },
        request_id,
    };

    // Store the text for later use in history
    state.pending_check_text = Some(text);
    eprintln!(
        "[DEBUG #{}] Prepared grammar request (revision={}, provider={}, model={})",
        request_id,
        state.editor_revision,
        state.config.provider.name(),
        state.config.model
    );
    log_grammar_state(state, "After preparing grammar request");

    match state.api_sender.send(request) {
        Ok(()) => {
            eprintln!("[DEBUG #{}] Grammar request sent to API worker", request_id);
        }
        Err(e) => {
            eprintln!(
                "[DEBUG #{}] Failed to send grammar request to worker: {}",
                request_id, e
            );
            state.status = format!("Internal error: failed to send request ({})", e);
            state.is_checking = false;
            state.current_check_request_id = None;
            state.current_check_revision = None;
            state.last_checked_revision = None;
            log_grammar_state(state, "After send failure");
        }
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
                    eprintln!(
                        "[DEBUG #{}] Received GrammarSuccess (suggestions={}), current_req={:?}, current_req_rev={:?}, editor_rev={}",
                        request_id,
                        suggestions.len(),
                        state.current_check_request_id,
                        state.current_check_revision,
                        state.editor_revision
                    );
                    if state.current_check_request_id != Some(request_id) {
                        eprintln!(
                            "[DEBUG #{}] Dropping GrammarSuccess as stale (expected current_req={:?})",
                            request_id,
                            state.current_check_request_id
                        );
                        continue;
                    }

                    // If text changed since the request was sent, drop this response.
                    if state.current_check_revision != Some(state.editor_revision) {
                        eprintln!(
                            "[DEBUG #{}] Dropping GrammarSuccess due to revision mismatch (request_rev={:?}, current_editor_rev={})",
                            request_id,
                            state.current_check_revision,
                            state.editor_revision
                        );
                        state.is_checking = false;
                        state.current_check_request_id = None;
                        state.current_check_revision = None;
                        state.pending_check_text = None;
                        log_grammar_state(state, "After dropping GrammarSuccess");
                        continue;
                    }

                    state.is_checking = false;
                    state.current_check_request_id = None;
                    state.current_check_revision = None;

                    // Save to history for cycle prevention
                    if let Some(user_text) = state.pending_check_text.take() {
                        let user_content = build_history_user_content(&user_text, &suggestions);
                        let assistant_content = build_history_assistant_content(&suggestions);
                        state
                            .message_history
                            .push_pair(user_content, assistant_content);
                    }

                    state.suggestions = suggestions;
                    rebuild_suggestion_highlight_cache(state);
                    refresh_highlight_settings(state);
                    if state.suggestions.is_empty() {
                        state.status = "All good!".to_string();
                    } else {
                        state.status = format!("{} suggestion(s)", state.suggestions.len());
                    }
                    eprintln!(
                        "[DEBUG #{}] Applied GrammarSuccess, status='{}'",
                        request_id, state.status
                    );
                    log_grammar_state(state, "After applying GrammarSuccess");
                }
                ApiResponse::GrammarError {
                    message,
                    request_id,
                } => {
                    eprintln!(
                        "[DEBUG #{}] Received GrammarError '{}', current_req={:?}, current_req_rev={:?}, editor_rev={}",
                        request_id,
                        message,
                        state.current_check_request_id,
                        state.current_check_revision,
                        state.editor_revision
                    );
                    if state.current_check_request_id != Some(request_id) {
                        eprintln!(
                            "[DEBUG #{}] Dropping GrammarError as stale (expected current_req={:?})",
                            request_id,
                            state.current_check_request_id
                        );
                        continue;
                    }

                    // If text changed since the request was sent, drop this error.
                    if state.current_check_revision != Some(state.editor_revision) {
                        eprintln!(
                            "[DEBUG #{}] Dropping GrammarError due to revision mismatch (request_rev={:?}, current_editor_rev={})",
                            request_id,
                            state.current_check_revision,
                            state.editor_revision
                        );
                        state.is_checking = false;
                        state.current_check_request_id = None;
                        state.current_check_revision = None;
                        state.pending_check_text = None;
                        log_grammar_state(state, "After dropping GrammarError");
                        continue;
                    }

                    state.is_checking = false;
                    state.current_check_request_id = None;
                    state.current_check_revision = None;
                    state.status = message;
                    eprintln!(
                        "[DEBUG #{}] Applied GrammarError, status='{}'",
                        request_id, state.status
                    );
                    log_grammar_state(state, "After applying GrammarError");
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
                ApiResponse::ModelsSuccess {
                    models,
                    provider,
                    request_id,
                } => {
                    if models_request_id_for_provider(state, &provider) != Some(request_id) {
                        continue;
                    }
                    clear_models_request_id_for_provider(state, &provider);

                    match provider {
                        ApiProvider::OpenAI => {
                            state.openai_models = models;
                            state.openai_models_loaded_for_key =
                                Some(state.temp_openai_api_key.trim().to_string());
                        }
                        ApiProvider::OpenRouter => {
                            state.openrouter_models = models;
                            state.openrouter_models_loaded_for_key =
                                Some(state.temp_openrouter_api_key.trim().to_string());
                        }
                        ApiProvider::Gemini => {
                            state.gemini_models = models;
                            state.gemini_models_loaded_for_key =
                                Some(state.temp_gemini_api_key.trim().to_string());
                        }
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
                ApiResponse::ModelsError {
                    message,
                    provider,
                    request_id,
                } => {
                    if models_request_id_for_provider(state, &provider) != Some(request_id) {
                        continue;
                    }
                    clear_models_request_id_for_provider(state, &provider);
                    eprintln!(
                        "[DEBUG] Failed to fetch models (provider={}): {}",
                        provider.name(),
                        message
                    );
                }
            },
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                eprintln!("[DEBUG] API response channel disconnected while processing responses");
                log_grammar_state(state, "Before handling API thread disconnect");
                state.is_checking = false;
                state.current_check_request_id = None;
                state.current_check_revision = None;
                state.pending_check_text = None;
                state.is_testing = false;
                state.current_test_request_id = None;
                state.openai_models_request_id = None;
                state.openrouter_models_request_id = None;
                state.gemini_models_request_id = None;
                state.models_fetch_debounce_start = None;
                state.status = "Internal error: API thread died".to_string();
                log_grammar_state(state, "After handling API thread disconnect");
                break;
            }
        }
    }
}

fn cancel_inflight_grammar_check(state: &mut State) {
    if !state.is_checking {
        eprintln!("[DEBUG] cancel_inflight_grammar_check called, but no check is active");
        return;
    }

    let cancelled_request_id = state.current_check_request_id;
    eprintln!(
        "[DEBUG] Cancelling in-flight grammar from UI side (request_id={:?}, request_rev={:?}, editor_rev={})",
        cancelled_request_id,
        state.current_check_revision,
        state.editor_revision
    );
    state.is_checking = false;
    state.current_check_request_id = None;
    state.current_check_revision = None;
    state.pending_check_text = None;
    log_grammar_state(state, "After clearing local in-flight grammar state");

    let request = ApiRequest {
        job: ApiJob::CancelGrammar,
        request_id: crate::api::next_request_id(),
    };
    match state.api_sender.send(request) {
        Ok(()) => {
            eprintln!(
                "[DEBUG] Sent CancelGrammar request for previous request_id={:?}",
                cancelled_request_id
            );
        }
        Err(e) => {
            eprintln!("[DEBUG] Failed to send CancelGrammar request: {}", e);
        }
    }
}

fn schedule_models_fetch(state: &mut State) {
    // Debounce model fetching so we don't hammer /models while the user types an API key.
    state.models_fetch_debounce_start = Some(Instant::now());
}

fn tick_models_fetch_debounce(state: &mut State) {
    if !state.show_settings {
        state.models_fetch_debounce_start = None;
        return;
    }

    let Some(start) = state.models_fetch_debounce_start else {
        return;
    };

    if start.elapsed() < Duration::from_millis(MODELS_FETCH_DEBOUNCE_MS) {
        return;
    }

    state.models_fetch_debounce_start = None;
    fetch_models_if_needed(state);
}

fn invalidate_models_cache_for_provider(state: &mut State, provider: ApiProvider) {
    let should_cancel = match provider {
        ApiProvider::OpenAI => state.openai_models_request_id.is_some(),
        ApiProvider::OpenRouter => state.openrouter_models_request_id.is_some(),
        ApiProvider::Gemini => state.gemini_models_request_id.is_some(),
    };

    if should_cancel {
        // Cancel any in-flight fetch and clear cached results to avoid showing models for the wrong key.
        let request = ApiRequest {
            job: ApiJob::CancelModels {
                provider: provider.clone(),
            },
            request_id: crate::api::next_request_id(),
        };
        let _ = state.api_sender.send(request);
    }

    match provider {
        ApiProvider::OpenAI => {
            state.openai_models.clear();
            state.openai_models_loaded_for_key = None;
            state.openai_models_request_id = None;
        }
        ApiProvider::OpenRouter => {
            state.openrouter_models.clear();
            state.openrouter_models_loaded_for_key = None;
            state.openrouter_models_request_id = None;
        }
        ApiProvider::Gemini => {
            state.gemini_models.clear();
            state.gemini_models_loaded_for_key = None;
            state.gemini_models_request_id = None;
        }
    }

    if provider == state.temp_provider {
        state.model_combo_state = iced::widget::combo_box::State::new(Vec::new());
    }
}

fn models_request_id_for_provider(state: &State, provider: &ApiProvider) -> Option<u64> {
    match provider {
        ApiProvider::OpenAI => state.openai_models_request_id,
        ApiProvider::OpenRouter => state.openrouter_models_request_id,
        ApiProvider::Gemini => state.gemini_models_request_id,
    }
}

fn clear_models_request_id_for_provider(state: &mut State, provider: &ApiProvider) {
    match provider {
        ApiProvider::OpenAI => state.openai_models_request_id = None,
        ApiProvider::OpenRouter => state.openrouter_models_request_id = None,
        ApiProvider::Gemini => state.gemini_models_request_id = None,
    }
}

fn clamp_to_char_boundary(s: &str, mut idx: usize, direction: i32) -> usize {
    idx = idx.min(s.len());
    while idx > 0 && idx < s.len() && !s.is_char_boundary(idx) {
        if direction < 0 {
            idx = idx.saturating_sub(1);
        } else {
            idx = (idx + 1).min(s.len());
        }
    }
    idx
}

fn truncate_string_middle(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        return s.to_string();
    }
    if max_len <= 3 {
        return ".".repeat(max_len);
    }

    let head_len = (max_len - 3) / 2;
    let tail_len = max_len - 3 - head_len;

    let head_end = clamp_to_char_boundary(s, head_len, -1);
    let tail_start = clamp_to_char_boundary(s, s.len().saturating_sub(tail_len), -1);

    let mut out = String::new();
    out.push_str(&s[..head_end]);
    out.push_str("...");
    out.push_str(&s[tail_start..]);
    out
}

fn excerpt_with_focus(text: &str, focus_start: usize, focus_end: usize, max_len: usize) -> String {
    if text.is_empty() || max_len == 0 {
        return String::new();
    }

    let focus_start = clamp_to_char_boundary(text, focus_start, -1);
    let focus_end = clamp_to_char_boundary(text, focus_end.min(text.len()), 1);
    if focus_end <= focus_start {
        return truncate_string_middle(text, max_len);
    }

    let focus_len = focus_end - focus_start;
    let markers_len = 4; // << >>

    if focus_len.saturating_add(markers_len) >= max_len {
        let focus = &text[focus_start..focus_end];
        return format!(
            "<<{}>>",
            truncate_string_middle(focus, max_len.saturating_sub(markers_len))
        );
    }

    let remaining = max_len - focus_len - markers_len;
    let before = remaining / 2;
    let after = remaining - before;

    let start = clamp_to_char_boundary(text, focus_start.saturating_sub(before), -1);
    let end = clamp_to_char_boundary(text, (focus_end + after).min(text.len()), 1);

    let mut snippet = String::new();
    if start > 0 {
        snippet.push_str("...");
    }
    snippet.push_str(&text[start..focus_start]);
    snippet.push_str("<<");
    snippet.push_str(&text[focus_start..focus_end]);
    snippet.push_str(">>");
    snippet.push_str(&text[focus_end..end]);
    if end < text.len() {
        snippet.push_str("...");
    }

    if snippet.len() > max_len {
        return truncate_string_middle(&snippet, max_len);
    }

    snippet
}

fn build_history_user_content(text: &str, suggestions: &[Suggestion]) -> String {
    let mut out = String::new();
    out.push_str(&format!("Text summary (len={}):\n", text.len()));

    if suggestions.is_empty() {
        out.push_str(&truncate_string_middle(
            text,
            HISTORY_NO_SUGGESTIONS_MAX_CHARS,
        ));
        return out;
    }

    out.push_str(&format!(
        "{} suggestion(s). Excerpts (showing up to {}):\n",
        suggestions.len(),
        HISTORY_MAX_EXCERPTS
    ));

    for (i, s) in suggestions.iter().take(HISTORY_MAX_EXCERPTS).enumerate() {
        let start = s.offset;
        let end = s.offset.saturating_add(s.length);
        let snippet = excerpt_with_focus(text, start, end, HISTORY_EXCERPT_MAX_CHARS);
        out.push_str(&format!("{}. {}\n", i + 1, snippet));
    }

    if suggestions.len() > HISTORY_MAX_EXCERPTS {
        out.push_str(&format!(
            "(plus {} more)",
            suggestions.len() - HISTORY_MAX_EXCERPTS
        ));
    }

    out
}

fn build_history_assistant_content(suggestions: &[Suggestion]) -> String {
    if suggestions.is_empty() {
        return r#"{"matches":[]}"#.to_string();
    }

    serde_json::to_string(&serde_json::json!({
        "matches": suggestions.iter().take(HISTORY_MAX_MATCHES).map(|s| {
            serde_json::json!({
                "message": truncate_string_middle(&s.message, HISTORY_FIELD_MAX_CHARS),
                "original": truncate_string_middle(&s.original, HISTORY_FIELD_MAX_CHARS),
                "replacement": s.replacement.as_ref().map(|r| truncate_string_middle(r, HISTORY_FIELD_MAX_CHARS)),
                "severity": format!("{:?}", s.severity).to_lowercase(),
            })
        }).collect::<Vec<_>>()
    }))
    .unwrap_or_else(|_| r#"{"matches":[]}"#.to_string())
}

fn fetch_models_if_needed(state: &mut State) {
    match state.temp_provider {
        ApiProvider::OpenAI => {
            let api_key = state.temp_openai_api_key.trim().to_string();
            if api_key.is_empty() {
                return;
            }

            if state.openai_models_request_id.is_some() {
                return;
            }

            if !state.openai_models.is_empty()
                && state.openai_models_loaded_for_key.as_deref() == Some(api_key.as_str())
            {
                state.model_combo_state =
                    iced::widget::combo_box::State::new(state.openai_models.clone());
                return;
            }

            let request_id = crate::api::next_request_id();
            state.openai_models_request_id = Some(request_id);
            let request = ApiRequest {
                job: ApiJob::FetchModels {
                    api_key,
                    provider: ApiProvider::OpenAI,
                },
                request_id,
            };
            let _ = state.api_sender.send(request);
        }
        ApiProvider::OpenRouter => {
            let api_key = state.temp_openrouter_api_key.trim().to_string();
            if api_key.is_empty() {
                return;
            }

            if state.openrouter_models_request_id.is_some() {
                return;
            }

            if !state.openrouter_models.is_empty()
                && state.openrouter_models_loaded_for_key.as_deref() == Some(api_key.as_str())
            {
                state.model_combo_state =
                    iced::widget::combo_box::State::new(state.openrouter_models.clone());
                return;
            }

            let request_id = crate::api::next_request_id();
            state.openrouter_models_request_id = Some(request_id);
            let request = ApiRequest {
                job: ApiJob::FetchModels {
                    api_key,
                    provider: ApiProvider::OpenRouter,
                },
                request_id,
            };
            let _ = state.api_sender.send(request);
        }
        ApiProvider::Gemini => {
            let api_key = state.temp_gemini_api_key.trim().to_string();
            if api_key.is_empty() {
                return;
            }

            if state.gemini_models_request_id.is_some() {
                return;
            }

            if !state.gemini_models.is_empty()
                && state.gemini_models_loaded_for_key.as_deref() == Some(api_key.as_str())
            {
                state.model_combo_state =
                    iced::widget::combo_box::State::new(state.gemini_models.clone());
                return;
            }

            let request_id = crate::api::next_request_id();
            state.gemini_models_request_id = Some(request_id);
            let request = ApiRequest {
                job: ApiJob::FetchModels {
                    api_key,
                    provider: ApiProvider::Gemini,
                },
                request_id,
            };
            let _ = state.api_sender.send(request);
        }
    }
}

fn rebuild_text_highlight_cache(state: &mut State) {
    state.highlight_line_starts = Arc::new(highlight::compute_line_starts(&state.editor_text));
    state.highlight_code_spans = Arc::new(highlight::spans_from_backticks(&state.editor_text));
}

fn rebuild_suggestion_highlight_cache(state: &mut State) {
    state.highlight_suggestion_spans = Arc::new(highlight::spans_from_suggestions(
        &state.suggestions,
        state.hovered_suggestion.as_deref(),
    ));
}

fn refresh_highlight_settings(state: &mut State) {
    state.highlight_settings = highlight::Settings {
        line_starts: state.highlight_line_starts.clone(),
        suggestion_spans: state.highlight_suggestion_spans.clone(),
        code_spans: state.highlight_code_spans.clone(),
    };
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

    let text = state.editor_text.as_str();
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
    state.editor_text = new_text;
    state.editor_revision = state.editor_revision.wrapping_add(1);
    rebuild_text_highlight_cache(state);
    rebuild_suggestion_highlight_cache(state);
    refresh_highlight_settings(state);
    state.last_checked_revision = Some(state.editor_revision);

    if state.suggestions.is_empty() {
        state.status = "All good!".to_string();
    } else {
        state.status = format!("{} suggestion(s)", state.suggestions.len());
    }
}

#[derive(Debug, Clone)]
struct EditorSnapshot {
    text: String,
    cursor: text_editor::Cursor,
}

fn restore_snapshot(state: &mut State, snapshot: EditorSnapshot) {
    let EditorSnapshot { text, cursor } = snapshot;
    state.editor = text_editor::Content::with_text(&text);
    state.editor.move_to(cursor);
    state.editor_text = text;
    state.editor_revision = state.editor_revision.wrapping_add(1);
    rebuild_text_highlight_cache(state);
    refresh_highlight_settings(state);
}
