use crate::config::{ApiProvider, Config};
use crate::suggestion::Suggestion;
use eframe::egui;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

use super::api_worker::spawn_api_worker;
use super::style::{setup_custom_style, COL_DANGER, COL_MUTED, COL_SUCCESS};

pub(super) const DEBOUNCE_MS: u64 = 800;

// Request message sent to the API thread
#[derive(Debug)]
pub(super) enum ApiJob {
    Grammar {
        text: String,
        api_key: String,
        model: String,
        provider: ApiProvider,
    },
    TestConnection {
        api_key: String,
        provider: ApiProvider,
    },
}

#[derive(Debug)]
pub(super) struct ApiRequest {
    pub(super) job: ApiJob,
    pub(super) request_id: u64,
}

// Response from API thread
#[derive(Debug)]
pub(super) enum ApiResponse {
    GrammarSuccess {
        suggestions: Vec<Suggestion>,
        request_id: u64,
    },
    GrammarError {
        message: String,
        request_id: u64,
    },
    TestSuccess {
        request_id: u64,
    },
    TestError {
        message: String,
        request_id: u64,
    },
}

pub struct GrammyApp {
    pub(super) text: String,
    pub(super) last_checked_text: String,
    pub(super) suggestions: Vec<Suggestion>,
    pub(super) status: String,
    pub(super) config: Config,
    pub(super) show_settings: bool,
    pub(super) temp_openai_api_key: String,
    pub(super) temp_openrouter_api_key: String,
    pub(super) temp_model: String,
    pub(super) temp_provider: ApiProvider,
    pub(super) show_api_key: bool,
    pub(super) test_status: String,
    pub(super) test_status_color: egui::Color32,
    pub(super) hovered_suggestion: Option<String>,
    pub(super) last_edit_time: Option<Instant>,
    pub(super) is_checking: bool,
    pub(super) current_check_request_id: Option<u64>,
    pub(super) pending_recheck: bool,
    pub(super) is_testing: bool,
    pub(super) current_test_request_id: Option<u64>,
    pub(super) api_sender: Sender<ApiRequest>,
    pub(super) api_receiver: Receiver<ApiResponse>,
}

impl GrammyApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_style(&cc.egui_ctx);

        let config = Config::load();
        let (request_tx, request_rx) = channel::<ApiRequest>();
        let (response_tx, response_rx) = channel::<ApiResponse>();

        spawn_api_worker(request_rx, response_tx);

        eprintln!(
            "[DEBUG] GrammyApp initialized, provider={}",
            config.provider.name()
        );

        Self {
            text: String::new(),
            last_checked_text: String::new(),
            suggestions: Vec::new(),
            status: "Ready".to_string(),
            config: config.clone(),
            show_settings: false,
            temp_openai_api_key: config.openai_api_key,
            temp_openrouter_api_key: config.openrouter_api_key,
            temp_model: config.model,
            temp_provider: config.provider,
            show_api_key: false,
            test_status: String::new(),
            test_status_color: COL_MUTED,
            hovered_suggestion: None,
            last_edit_time: None,
            is_checking: false,
            current_check_request_id: None,
            pending_recheck: false,
            is_testing: false,
            current_test_request_id: None,
            api_sender: request_tx,
            api_receiver: response_rx,
        }
    }

    pub(super) fn schedule_check(&mut self) {
        eprintln!(
            "[DEBUG] Scheduling check, current text len={}",
            self.text.len()
        );
        self.last_edit_time = Some(Instant::now());
        if self.is_checking {
            self.pending_recheck = true;
        }
    }

    pub(super) fn check_text(&mut self) {
        if self.text.trim().is_empty() {
            eprintln!("[DEBUG] check_text: empty text, clearing suggestions");
            self.suggestions.clear();
            self.status = "Ready".to_string();
            self.last_checked_text = self.text.clone();
            self.is_checking = false;
            self.current_check_request_id = None;
            return;
        }

        // Don't re-check if text hasn't changed
        if self.text == self.last_checked_text {
            eprintln!("[DEBUG] check_text: text unchanged, skipping");
            return;
        }

        if self.is_checking {
            eprintln!("[DEBUG] check_text: request in-flight, queueing recheck");
            self.pending_recheck = true;
            return;
        }

        // Generate a new request ID - this invalidates any in-flight requests
        let request_id = crate::api::next_request_id();
        eprintln!(
            "[DEBUG] check_text: starting request #{}, text_len={}",
            request_id,
            self.text.len()
        );

        self.is_checking = true;
        self.current_check_request_id = Some(request_id);
        self.status = "Checking...".to_string();
        self.last_checked_text = self.text.clone();

        let request = ApiRequest {
            job: ApiJob::Grammar {
                text: self.text.clone(),
                api_key: self.config.api_key_for_provider(&self.config.provider),
                model: self.config.model.clone(),
                provider: self.config.provider.clone(),
            },
            request_id,
        };

        if let Err(e) = self.api_sender.send(request) {
            eprintln!("[DEBUG] Failed to send request: {}", e);
            self.status = "Internal error: failed to send request".to_string();
            self.is_checking = false;
            self.current_check_request_id = None;
        }
    }

    pub(super) fn process_api_responses(&mut self) {
        // Process all pending responses
        loop {
            match self.api_receiver.try_recv() {
                Ok(response) => match response {
                    ApiResponse::GrammarSuccess {
                        suggestions,
                        request_id,
                    } => {
                        if self.current_check_request_id != Some(request_id) {
                            eprintln!(
                                "[DEBUG] Discarding stale grammar response #{} (current={})",
                                request_id,
                                self.current_check_request_id
                                    .map(|id| id.to_string())
                                    .unwrap_or("none".into())
                            );
                            continue;
                        }

                        eprintln!(
                            "[DEBUG] Processing grammar success response #{}",
                            request_id
                        );
                        self.is_checking = false;
                        self.current_check_request_id = None;

                        self.suggestions = suggestions;
                        if self.suggestions.is_empty() {
                            self.status = "All good!".to_string();
                        } else {
                            self.status = format!("{} suggestion(s)", self.suggestions.len());
                        }

                        if self.pending_recheck {
                            self.pending_recheck = false;
                            self.last_edit_time = Some(
                                Instant::now() - Duration::from_millis(DEBOUNCE_MS),
                            );
                        }
                    }
                    ApiResponse::GrammarError { message, request_id } => {
                        if self.current_check_request_id != Some(request_id) {
                            eprintln!(
                                "[DEBUG] Discarding stale grammar error #{} (current={})",
                                request_id,
                                self.current_check_request_id
                                    .map(|id| id.to_string())
                                    .unwrap_or("none".into())
                            );
                            continue;
                        }

                        eprintln!("[DEBUG] Processing grammar error response #{}", request_id);
                        self.is_checking = false;
                        self.current_check_request_id = None;
                        self.status = message;

                        if self.pending_recheck {
                            self.pending_recheck = false;
                            self.last_edit_time = Some(
                                Instant::now() - Duration::from_millis(DEBOUNCE_MS),
                            );
                        }
                    }
                    ApiResponse::TestSuccess { request_id } => {
                        if self.current_test_request_id != Some(request_id) {
                            eprintln!(
                                "[DEBUG] Discarding stale test response #{} (current={})",
                                request_id,
                                self.current_test_request_id
                                    .map(|id| id.to_string())
                                    .unwrap_or("none".into())
                            );
                            continue;
                        }

                        eprintln!("[DEBUG] Processing test success response #{}", request_id);
                        self.is_testing = false;
                        self.current_test_request_id = None;
                        self.test_status = "Connection OK".to_string();
                        self.test_status_color = COL_SUCCESS;
                    }
                    ApiResponse::TestError { message, request_id } => {
                        if self.current_test_request_id != Some(request_id) {
                            eprintln!(
                                "[DEBUG] Discarding stale test error #{} (current={})",
                                request_id,
                                self.current_test_request_id
                                    .map(|id| id.to_string())
                                    .unwrap_or("none".into())
                            );
                            continue;
                        }

                        eprintln!("[DEBUG] Processing test error response #{}", request_id);
                        self.is_testing = false;
                        self.current_test_request_id = None;
                        self.test_status = message;
                        self.test_status_color = COL_DANGER;
                    }
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    eprintln!("[DEBUG] API channel disconnected!");
                    self.status = "Internal error: API thread died".to_string();
                    break;
                }
            }
        }
    }

    pub(super) fn start_test_connection(&mut self) {
        if self.is_testing {
            return;
        }
        let request_id = crate::api::next_request_id();
        self.is_testing = true;
        self.current_test_request_id = Some(request_id);
        self.test_status = "Testing...".to_string();
        self.test_status_color = COL_MUTED;

        let api_key = match self.temp_provider {
            ApiProvider::OpenAI => self.temp_openai_api_key.trim().to_string(),
            ApiProvider::OpenRouter => self.temp_openrouter_api_key.trim().to_string(),
        };

        let request = ApiRequest {
            job: ApiJob::TestConnection {
                api_key,
                provider: self.temp_provider.clone(),
            },
            request_id,
        };

        if let Err(e) = self.api_sender.send(request) {
            eprintln!("[DEBUG] Failed to send test request: {}", e);
            self.is_testing = false;
            self.current_test_request_id = None;
            self.test_status = "Internal error: failed to send test".to_string();
            self.test_status_color = COL_DANGER;
        }
    }

    pub(super) fn apply_suggestion(&mut self, suggestion_id: &str) {
        let suggestion = self
            .suggestions
            .iter()
            .find(|s| s.id == suggestion_id)
            .cloned();

        if let Some(suggestion) = suggestion {
            let start = suggestion.offset;
            let end = suggestion.offset + suggestion.length;

            if start > self.text.len() || end > self.text.len() {
                self.status = "Invalid suggestion range".to_string();
                self.schedule_check();
                return;
            }

            let slice = &self.text[start..end];
            if slice != suggestion.original {
                self.status = "Text changed; re-checking...".to_string();
                self.schedule_check();
                return;
            }

            let new_text = format!(
                "{}{}{}",
                &self.text[..start],
                suggestion.replacement,
                &self.text[end..]
            );

            let delta = suggestion.replacement.len() as isize - suggestion.length as isize;

            self.suggestions.retain(|s| s.id != suggestion_id);
            for s in &mut self.suggestions {
                if s.offset > suggestion.offset {
                    s.offset = (s.offset as isize + delta) as usize;
                }
            }

            self.text = new_text.clone();
            self.last_checked_text = new_text;

            if self.suggestions.is_empty() {
                self.status = "All good!".to_string();
            } else {
                self.status = format!("{} suggestion(s)", self.suggestions.len());
            }
        }
    }

    pub(super) fn save_settings(&mut self) {
        // Persist both keys regardless of current provider
        self.config.openai_api_key = self.temp_openai_api_key.trim().to_string();
        self.config.openrouter_api_key = self.temp_openrouter_api_key.trim().to_string();
        self.config.provider = self.temp_provider.clone();
        self.config.model = if self.temp_model.trim().is_empty() {
            self.config.provider.default_model().to_string()
        } else {
            self.temp_model.trim().to_string()
        };
        self.config.save();
        self.show_settings = false;
        self.status = "Settings saved".to_string();
        eprintln!(
            "[DEBUG] Settings saved: provider={}, model={}",
            self.config.provider.name(),
            self.config.model
        );
    }
}
