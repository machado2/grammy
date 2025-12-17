use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

use iced::widget::text_editor;
use iced::{Subscription, Task, Theme};

use crate::config::{ApiProvider, Config};
use crate::suggestion::Suggestion;

use super::api_worker::{spawn_api_worker, ApiJob, ApiRequest, ApiResponse};
use super::style;
use super::ui;

const DEBOUNCE_MS: u64 = 800;
const TICK_MS: u64 = 50;

#[derive(Debug, Clone)]
pub enum Message {
    Tick,

    EditorAction(text_editor::Action),
    ApplySuggestion(String),

    OpenSettings,
    CloseSettings,
    ToggleShowApiKey,

    SelectProvider(ApiProvider),
    TempOpenAiKeyChanged(String),
    TempOpenRouterKeyChanged(String),
    TempModelChanged(String),

    SaveSettings,
    StartTestConnection,
}

pub struct State {
    pub(super) editor: text_editor::Content,
    pub(super) last_checked_text: String,
    pub(super) suggestions: Vec<Suggestion>,

    pub(super) status: String,

    pub(super) config: Config,

    pub(super) show_settings: bool,
    pub(super) show_api_key: bool,
    pub(super) temp_openai_api_key: String,
    pub(super) temp_openrouter_api_key: String,
    pub(super) temp_model: String,
    pub(super) temp_provider: ApiProvider,

    pub(super) test_status: String,
    pub(super) is_testing: bool,
    pub(super) current_test_request_id: Option<u64>,

    pub(super) last_edit_time: Option<Instant>,
    pub(super) is_checking: bool,
    pub(super) current_check_request_id: Option<u64>,
    pub(super) pending_recheck: bool,

    pub(super) api_sender: Sender<ApiRequest>,
    pub(super) api_receiver: Receiver<ApiResponse>,
}

pub fn new() -> (State, Task<Message>) {
    let config = Config::load();

    let (request_tx, request_rx) = channel::<ApiRequest>();
    let (response_tx, response_rx) = channel::<ApiResponse>();
    spawn_api_worker(request_rx, response_tx);

    let editor = text_editor::Content::new();

    (
        State {
            editor,
            last_checked_text: String::new(),
            suggestions: Vec::new(),
            status: "Ready".to_string(),
            config: config.clone(),
            show_settings: false,
            show_api_key: false,
            temp_openai_api_key: config.openai_api_key,
            temp_openrouter_api_key: config.openrouter_api_key,
            temp_model: config.model,
            temp_provider: config.provider,
            test_status: String::new(),
            is_testing: false,
            current_test_request_id: None,
            last_edit_time: None,
            is_checking: false,
            current_check_request_id: None,
            pending_recheck: false,
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

        Message::EditorAction(action) => {
            state.editor.perform(action);
            state.suggestions.clear();
            state.last_edit_time = Some(Instant::now());
            if state.is_checking {
                state.pending_recheck = true;
            }
            Task::none()
        }

        Message::ApplySuggestion(id) => {
            apply_suggestion(state, &id);
            Task::none()
        }

        Message::OpenSettings => {
            state.temp_openai_api_key = state.config.openai_api_key.clone();
            state.temp_openrouter_api_key = state.config.openrouter_api_key.clone();
            state.temp_model = state.config.model.clone();
            state.temp_provider = state.config.provider.clone();
            state.show_api_key = false;
            state.test_status.clear();
            state.show_settings = true;
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
            if matches!(p, ApiProvider::OpenAI) {
                if state.temp_model.is_empty() || state.temp_model.starts_with("openai/") {
                    state.temp_model = ApiProvider::OpenAI.default_model().to_string();
                }
            } else if state.temp_model.is_empty() || !state.temp_model.contains('/') {
                state.temp_model = ApiProvider::OpenRouter.default_model().to_string();
            }
            state.test_status.clear();
            state.show_api_key = false;
            Task::none()
        }

        Message::TempOpenAiKeyChanged(v) => {
            state.temp_openai_api_key = v;
            Task::none()
        }
        Message::TempOpenRouterKeyChanged(v) => {
            state.temp_openrouter_api_key = v;
            Task::none()
        }
        Message::TempModelChanged(v) => {
            state.temp_model = v;
            Task::none()
        }

        Message::SaveSettings => {
            state.config.openai_api_key = state.temp_openai_api_key.trim().to_string();
            state.config.openrouter_api_key = state.temp_openrouter_api_key.trim().to_string();
            state.config.provider = state.temp_provider.clone();
            state.config.model = if state.temp_model.trim().is_empty() {
                state.config.provider.default_model().to_string()
            } else {
                state.temp_model.trim().to_string()
            };
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
            };

            let request = ApiRequest {
                job: ApiJob::TestConnection {
                    api_key,
                    provider: state.temp_provider.clone(),
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
    iced::time::every(Duration::from_millis(TICK_MS)).map(|_| Message::Tick)
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
        if edit_time.elapsed() >= Duration::from_millis(DEBOUNCE_MS) {
            state.last_edit_time = None;
            check_text(state);
        }
    }
}

fn check_text(state: &mut State) {
    let text = state.editor.text();

    if text.trim().is_empty() {
        state.suggestions.clear();
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
    state.last_checked_text = text.clone();

    let request = ApiRequest {
        job: ApiJob::Grammar {
            text,
            api_key: state.config.api_key_for_provider(&state.config.provider),
            model: state.config.model.clone(),
            provider: state.config.provider.clone(),
        },
        request_id,
    };

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

                    state.suggestions = suggestions;
                    if state.suggestions.is_empty() {
                        state.status = "All good!".to_string();
                    } else {
                        state.status = format!("{} suggestion(s)", state.suggestions.len());
                    }

                    if state.pending_recheck {
                        state.pending_recheck = false;
                        state.last_edit_time = Some(
                            Instant::now() - Duration::from_millis(DEBOUNCE_MS),
                        );
                    }
                }
                ApiResponse::GrammarError { message, request_id } => {
                    if state.current_check_request_id != Some(request_id) {
                        continue;
                    }

                    state.is_checking = false;
                    state.current_check_request_id = None;
                    state.status = message;

                    if state.pending_recheck {
                        state.pending_recheck = false;
                        state.last_edit_time = Some(
                            Instant::now() - Duration::from_millis(DEBOUNCE_MS),
                        );
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
                ApiResponse::TestError { message, request_id } => {
                    if state.current_test_request_id != Some(request_id) {
                        continue;
                    }

                    state.is_testing = false;
                    state.current_test_request_id = None;
                    state.test_status = message;
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

    let new_text = format!("{}{}{}", &text[..start], suggestion.replacement, &text[end..]);

    let delta = suggestion.replacement.len() as isize - suggestion.length as isize;

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
