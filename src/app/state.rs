use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

use eframe::{egui, Storage};
use egui::{Color32, FontFamily, FontId, Rounding, TextStyle};

use crate::config::{ApiProvider, Config};
use crate::suggestion::Suggestion;

use super::api_worker::{spawn_api_worker, ApiJob, ApiRequest, ApiResponse};
use super::draft;
use super::history::MessageHistory;

const AUTOSAVE_SECS: u64 = 30;

pub struct App {
    pub editor_text: String,
    pub last_checked_text: String,
    pub suggestions: Vec<Suggestion>,
    
    pub draft_dirty: bool,
    pub hovered_suggestion_id: Option<String>,
    
    pub status: String,
    
    pub config: Config,
    
    pub show_settings: bool,
    pub show_api_key: bool,
    pub temp_openai_api_key: String,
    pub temp_openrouter_api_key: String,
    pub temp_gemini_api_key: String,
    pub temp_model: String,
    pub temp_provider: ApiProvider,
    pub temp_debounce_ms: f32,
    
    pub openai_models: Vec<String>,
    pub openrouter_models: Vec<String>,
    pub gemini_models: Vec<String>,
    
    pub test_status: String,
    pub is_testing: bool,
    pub current_test_request_id: Option<u64>,
    
    pub last_edit_time: Option<Instant>,
    pub is_checking: bool,
    pub current_check_request_id: Option<u64>,
    pub pending_recheck: bool,
    pub pending_check_text: Option<String>,
    
    pub message_history: MessageHistory,
    
    api_sender: Sender<ApiRequest>,
    api_receiver: Receiver<ApiResponse>,
    
    pub last_autosave: Option<Instant>,
    
    pub editor_id: egui::Id,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Set egui style
        configure_egui_style(&cc.egui_ctx);
        
        let config = Config::load();
        
        let (request_tx, request_rx) = channel::<ApiRequest>();
        let (response_tx, response_rx) = channel::<ApiResponse>();
        spawn_api_worker(request_rx, response_tx);
        
        let draft = draft::load();
        let editor_text = draft.text;
        
        Self {
            editor_text,
            last_checked_text: String::new(),
            suggestions: Vec::new(),
            
            draft_dirty: false,
            hovered_suggestion_id: None,
            status: "Ready".to_string(),
            config: config.clone(),
            show_settings: false,
            show_api_key: false,
            temp_openai_api_key: config.openai_api_key.clone(),
            temp_openrouter_api_key: config.openrouter_api_key.clone(),
            temp_gemini_api_key: config.gemini_api_key.clone(),
            temp_model: config.model.clone(),
            temp_provider: config.provider.clone(),
            temp_debounce_ms: config.debounce_ms as f32,
            
            openai_models: Vec::new(),
            openrouter_models: Vec::new(),
            gemini_models: Vec::new(),
            
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
            last_autosave: Some(Instant::now()),
            editor_id: egui::Id::new("editor"),
        }
    }
    
    fn process_api_responses(&mut self) {
        loop {
            match self.api_receiver.try_recv() {
                Ok(response) => match response {
                    ApiResponse::GrammarSuccess {
                        suggestions,
                        request_id,
                    } => {
                        if self.current_check_request_id != Some(request_id) {
                            continue;
                        }
                        
                        self.is_checking = false;
                        self.current_check_request_id = None;
                        
                        // Save to history for cycle prevention
                        if let Some(user_text) = self.pending_check_text.take() {
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
                            self.message_history
                                .push_pair(format!("Text:\n{}", user_text), assistant_content);
                        }
                        
                        self.suggestions = suggestions;
                        if self.suggestions.is_empty() {
                            self.status = "All good!".to_string();
                        } else {
                            self.status = format!("{} suggestion(s)", self.suggestions.len());
                        }
                        
                        if self.pending_recheck {
                            let delay = self.config.debounce_ms;
                            if delay <= 5000 {
                                self.last_edit_time =
                                    Some(Instant::now() - Duration::from_millis(delay));
                            } else {
                                self.pending_recheck = false;
                            }
                        }
                    }
                    ApiResponse::GrammarError {
                        message,
                        request_id,
                    } => {
                        if self.current_check_request_id != Some(request_id) {
                            continue;
                        }
                        
                        self.is_checking = false;
                        self.current_check_request_id = None;
                        self.status = message;
                        
                        if self.pending_recheck {
                            let delay = self.config.debounce_ms;
                            if delay <= 5000 {
                                self.last_edit_time =
                                    Some(Instant::now() - Duration::from_millis(delay));
                            } else {
                                self.pending_recheck = false;
                            }
                        }
                    }
                    ApiResponse::TestSuccess { request_id } => {
                        if self.current_test_request_id != Some(request_id) {
                            continue;
                        }
                        
                        self.is_testing = false;
                        self.current_test_request_id = None;
                        self.test_status = "Connection OK".to_string();
                    }
                    ApiResponse::TestError {
                        message,
                        request_id,
                    } => {
                        if self.current_test_request_id != Some(request_id) {
                            continue;
                        }
                        
                        self.is_testing = false;
                        self.current_test_request_id = None;
                        self.test_status = message;
                    }
                    ApiResponse::ModelsSuccess { models, provider } => {
                        match provider {
                            ApiProvider::OpenAI => self.openai_models = models,
                            ApiProvider::OpenRouter => self.openrouter_models = models,
                            ApiProvider::Gemini => self.gemini_models = models,
                        }
                    }
                    ApiResponse::ModelsError { message } => {
                        eprintln!("[DEBUG] Failed to fetch models: {}", message);
                    }
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.status = "Internal error: API thread died".to_string();
                    break;
                }
            }
        }
    }
    
    fn tick_debounce(&mut self) {
        if self.last_edit_time.is_none() {
            return;
        }
        
        if let Some(edit_time) = self.last_edit_time {
            let delay = self.config.debounce_ms;
            if delay > 5000 {
                return;
            }
            if edit_time.elapsed() >= Duration::from_millis(delay) {
                self.last_edit_time = None;
                self.check_text();
            }
        }
    }
    
    pub fn check_text(&mut self) {
        let text = self.editor_text.clone();
        
        if text.trim().is_empty() {
            self.suggestions.clear();
            self.hovered_suggestion_id = None;
            self.status = "Ready".to_string();
            self.last_checked_text = text;
            self.is_checking = false;
            self.current_check_request_id = None;
            return;
        }
        
        if text == self.last_checked_text {
            return;
        }
        
        if self.is_checking {
            self.pending_recheck = true;
            return;
        }
        
        let request_id = crate::api::next_request_id();
        
        self.is_checking = true;
        self.current_check_request_id = Some(request_id);
        self.status = "Checking...".to_string();
        
        self.suggestions.clear();
        self.hovered_suggestion_id = None;
        self.last_checked_text = text.clone();
        
        let request = ApiRequest {
            job: ApiJob::Grammar {
                text: text.clone(),
                api_key: self.config.api_key_for_provider(&self.config.provider),
                model: self.config.model.clone(),
                provider: self.config.provider.clone(),
                history: self
                    .message_history
                    .get_entries()
                    .into_iter()
                    .cloned()
                    .collect(),
            },
            request_id,
        };
        
        self.pending_check_text = Some(text);
        
        if let Err(e) = self.api_sender.send(request) {
            self.status = format!("Internal error: failed to send request ({})", e);
            self.is_checking = false;
            self.current_check_request_id = None;
        }
    }
    
    pub fn apply_suggestion(&mut self, suggestion_id: &str) {
        let suggestion = self
            .suggestions
            .iter()
            .find(|s| s.id == suggestion_id)
            .cloned();
        
        let Some(suggestion) = suggestion else {
            return;
        };
        
        let text = &self.editor_text;
        let start = suggestion.offset;
        let end = suggestion.offset + suggestion.length;
        
        if start > text.len() || end > text.len() {
            self.status = "Invalid suggestion range".to_string();
            self.last_edit_time = Some(Instant::now());
            return;
        }
        
        let slice = &text[start..end];
        if slice != suggestion.original {
            self.status = "Text changed; re-checking...".to_string();
            self.last_edit_time = Some(Instant::now());
            return;
        }
        
        let replacement = match &suggestion.replacement {
            Some(r) => r,
            None => return,
        };
        
        let new_text = format!("{}{}{}", &text[..start], replacement, &text[end..]);
        
        let delta = replacement.len() as isize - suggestion.length as isize;
        
        self.suggestions.retain(|s| s.id != suggestion_id);
        for s in &mut self.suggestions {
            if s.offset > suggestion.offset {
                s.offset = (s.offset as isize + delta) as usize;
            }
        }
        
        self.last_checked_text = new_text.clone();
        self.editor_text = new_text;
        self.draft_dirty = true;
        
        if self.suggestions.is_empty() {
            self.status = "All good!".to_string();
        } else {
            self.status = format!("{} suggestion(s)", self.suggestions.len());
        }
    }
    
    pub fn fetch_models_if_needed(&mut self) {
        let api_key = match self.temp_provider {
            ApiProvider::OpenAI => &self.temp_openai_api_key,
            ApiProvider::OpenRouter => &self.temp_openrouter_api_key,
            ApiProvider::Gemini => &self.temp_gemini_api_key,
        };
        
        if api_key.is_empty() {
            return;
        }
        
        let has_models = match self.temp_provider {
            ApiProvider::OpenAI => !self.openai_models.is_empty(),
            ApiProvider::OpenRouter => !self.openrouter_models.is_empty(),
            ApiProvider::Gemini => !self.gemini_models.is_empty(),
        };
        
        if has_models {
            return;
        }
        
        let request_id = crate::api::next_request_id();
        let request = ApiRequest {
            job: ApiJob::FetchModels {
                api_key: api_key.clone(),
                provider: self.temp_provider.clone(),
            },
            request_id,
        };
        
        let _ = self.api_sender.send(request);
    }
    
    pub fn test_connection(&mut self) {
        if self.is_testing {
            return;
        }
        
        let request_id = crate::api::next_request_id();
        self.is_testing = true;
        self.current_test_request_id = Some(request_id);
        self.test_status = "Testing...".to_string();
        
        let api_key = match self.temp_provider {
            ApiProvider::OpenAI => self.temp_openai_api_key.trim().to_string(),
            ApiProvider::OpenRouter => self.temp_openrouter_api_key.trim().to_string(),
            ApiProvider::Gemini => self.temp_gemini_api_key.trim().to_string(),
        };
        
        let request = ApiRequest {
            job: ApiJob::TestConnection {
                api_key,
                provider: self.temp_provider.clone(),
                model: self.temp_model.clone(),
            },
            request_id,
        };
        
        if let Err(e) = self.api_sender.send(request) {
            self.is_testing = false;
            self.current_test_request_id = None;
            self.test_status = format!("Internal error: failed to send test ({})", e);
        }
    }
    
    pub fn save_settings(&mut self) {
        self.config.openai_api_key = self.temp_openai_api_key.trim().to_string();
        self.config.openrouter_api_key = self.temp_openrouter_api_key.trim().to_string();
        self.config.gemini_api_key = self.temp_gemini_api_key.trim().to_string();
        self.config.provider = self.temp_provider.clone();
        self.config.model = if self.temp_model.trim().is_empty() {
            self.config.provider.default_model().to_string()
        } else {
            self.temp_model.trim().to_string()
        };
        self.config.debounce_ms = self.temp_debounce_ms as u64;
        self.config.save();
        self.show_settings = false;
        self.status = "Settings saved".to_string();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process API responses
        self.process_api_responses();
        
        // Handle debounce timer
        ctx.request_repaint_after(Duration::from_millis(50));
        self.tick_debounce();
        
        // Handle autosave
        if let Some(last_autosave) = self.last_autosave {
            if last_autosave.elapsed() >= Duration::from_secs(AUTOSAVE_SECS) {
                if self.draft_dirty {
                    draft::save_text(self.editor_text.clone());
                    self.draft_dirty = false;
                }
                self.last_autosave = Some(Instant::now());
            }
        }
        
        // Show UI
        super::ui::show(ctx, self);
    }
    
    fn save(&mut self, storage: &mut dyn Storage) {
        // Save any persistent state if needed
        eframe::set_value(storage, eframe::APP_KEY, &self.editor_text);
    }
}

fn configure_egui_style(ctx: &egui::Context) {
    // Set up a warm, editorial theme with strong contrast.
    let mut style = (*ctx.style()).clone();
    
    style.visuals.dark_mode = false;
    style.visuals.panel_fill = Color32::from_rgb(245, 241, 234);
    style.visuals.window_fill = Color32::from_rgb(255, 255, 255);
    style.visuals.faint_bg_color = Color32::from_rgb(250, 246, 240);
    style.visuals.extreme_bg_color = Color32::from_rgb(236, 229, 219);
    style.visuals.code_bg_color = Color32::from_rgb(248, 244, 238);
    
    style.visuals.override_text_color = Some(Color32::from_rgb(28, 33, 37));
    style.visuals.selection.bg_fill = Color32::from_rgba_premultiplied(26, 134, 108, 70);
    style.visuals.selection.stroke.color = Color32::from_rgb(15, 109, 92);
    
    style.visuals.widgets.inactive.fg_stroke.color = Color32::from_rgb(40, 46, 52);
    style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(255, 255, 255);
    style.visuals.widgets.inactive.bg_stroke.color =
        Color32::from_rgba_premultiplied(36, 33, 28, 30);
    style.visuals.widgets.inactive.rounding = Rounding::same(12.0);
    style.visuals.widgets.hovered.fg_stroke.color = Color32::from_rgb(25, 28, 36);
    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(236, 244, 241);
    style.visuals.widgets.hovered.rounding = Rounding::same(12.0);
    style.visuals.widgets.active.fg_stroke.color = Color32::from_rgb(25, 28, 36);
    style.visuals.widgets.active.bg_fill = Color32::from_rgb(222, 236, 231);
    style.visuals.widgets.active.bg_stroke.color =
        Color32::from_rgba_premultiplied(36, 33, 28, 80);
    style.visuals.widgets.active.rounding = Rounding::same(12.0);
    style.visuals.widgets.noninteractive.rounding = Rounding::same(12.0);
    style.visuals.window_rounding = Rounding::same(18.0);
    style.visuals.menu_rounding = Rounding::same(14.0);
    
    style.visuals.text_cursor.stroke.width = 2.0;
    style.visuals.window_shadow = egui::epaint::Shadow::NONE;
    
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    style.spacing.item_spacing = egui::vec2(12.0, 12.0);
    style.spacing.window_margin = egui::Margin::same(14.0);
    style.spacing.interact_size = egui::vec2(40.0, 32.0);
    
    style.text_styles = [
        (TextStyle::Heading, FontId::new(28.0, FontFamily::Proportional)),
        (TextStyle::Name("Title".into()), FontId::new(20.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(14.5, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(13.5, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(13.5, FontFamily::Monospace)),
    ]
    .into();
    
    ctx.set_style(style);
}
