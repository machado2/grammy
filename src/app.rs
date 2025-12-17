use crate::api;
use crate::config::{ApiProvider, Config};
use crate::suggestion::Suggestion;
use eframe::egui;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

const DEBOUNCE_MS: u64 = 800;

// Color palette
const COL_BG: egui::Color32 = egui::Color32::from_rgb(11, 16, 32);
const COL_PANEL: egui::Color32 = egui::Color32::from_rgb(18, 26, 50);
const COL_EDITOR_BG: egui::Color32 = egui::Color32::from_rgb(15, 23, 48);
const COL_TEXT: egui::Color32 = egui::Color32::from_rgb(232, 236, 255);
const COL_MUTED: egui::Color32 = egui::Color32::from_rgb(169, 178, 211);
const COL_ACCENT: egui::Color32 = egui::Color32::from_rgb(110, 168, 254);
const COL_SUCCESS: egui::Color32 = egui::Color32::from_rgb(126, 231, 135);
const COL_DANGER: egui::Color32 = egui::Color32::from_rgb(255, 107, 107);

// Request message sent to the API thread
#[derive(Debug)]
struct ApiRequest {
    text: String,
    api_key: String,
    model: String,
    provider: ApiProvider,
    request_id: u64,
}

// Response from API thread
#[derive(Debug)]
enum ApiResponse {
    Success { suggestions: Vec<Suggestion>, request_id: u64 },
    Error { message: String, request_id: u64 },
}

pub struct GrammyApp {
    text: String,
    last_checked_text: String,
    suggestions: Vec<Suggestion>,
    status: String,
    config: Config,
    show_settings: bool,
    temp_api_key: String,
    temp_model: String,
    temp_provider: ApiProvider,
    hovered_suggestion: Option<String>,
    last_edit_time: Option<Instant>,
    is_checking: bool,
    current_request_id: Option<u64>,
    api_sender: Sender<ApiRequest>,
    api_receiver: Receiver<ApiResponse>,
}

impl GrammyApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_style(&cc.egui_ctx);

        let config = Config::load();
        let (request_tx, request_rx) = channel::<ApiRequest>();
        let (response_tx, response_rx) = channel::<ApiResponse>();

        // Spawn API handler thread
        std::thread::spawn(move || {
            eprintln!("[DEBUG] API thread started");
            let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");
            
            while let Ok(req) = request_rx.recv() {
                eprintln!("[DEBUG] API thread received request #{}", req.request_id);
                let tx = response_tx.clone();
                let request_id = req.request_id;
                
                rt.block_on(async {
                    match api::check_grammar(
                        req.text,
                        req.api_key,
                        req.model,
                        req.provider,
                        request_id,
                    ).await {
                        Ok((suggestions, req_id)) => {
                            eprintln!("[DEBUG] API thread sending success response for #{}", req_id);
                            let _ = tx.send(ApiResponse::Success { suggestions, request_id: req_id });
                        }
                        Err(e) => {
                            eprintln!("[DEBUG] API thread sending error response for #{}: {}", request_id, e);
                            let _ = tx.send(ApiResponse::Error { message: e, request_id });
                        }
                    }
                });
            }
            eprintln!("[DEBUG] API thread exiting");
        });

        eprintln!("[DEBUG] GrammyApp initialized, provider={}", config.provider.name());

        Self {
            text: String::new(),
            last_checked_text: String::new(),
            suggestions: Vec::new(),
            status: "Ready".to_string(),
            config: config.clone(),
            show_settings: false,
            temp_api_key: config.api_key,
            temp_model: config.model,
            temp_provider: config.provider,
            hovered_suggestion: None,
            last_edit_time: None,
            is_checking: false,
            current_request_id: None,
            api_sender: request_tx,
            api_receiver: response_rx,
        }
    }

    fn schedule_check(&mut self) {
        eprintln!("[DEBUG] Scheduling check, current text len={}", self.text.len());
        self.last_edit_time = Some(Instant::now());
        
        // If we're currently checking, the result will be discarded when it arrives
        // because current_request_id will change
    }

    fn check_text(&mut self) {
        if self.text.trim().is_empty() {
            eprintln!("[DEBUG] check_text: empty text, clearing suggestions");
            self.suggestions.clear();
            self.status = "Ready".to_string();
            self.last_checked_text = self.text.clone();
            self.is_checking = false;
            self.current_request_id = None;
            return;
        }

        // Don't re-check if text hasn't changed
        if self.text == self.last_checked_text {
            eprintln!("[DEBUG] check_text: text unchanged, skipping");
            return;
        }

        // Generate a new request ID - this invalidates any in-flight requests
        let request_id = api::next_request_id();
        eprintln!("[DEBUG] check_text: starting request #{}, text_len={}", request_id, self.text.len());
        
        self.is_checking = true;
        self.current_request_id = Some(request_id);
        self.status = "Checking...".to_string();
        self.last_checked_text = self.text.clone();

        let request = ApiRequest {
            text: self.text.clone(),
            api_key: self.config.api_key.clone(),
            model: self.config.model.clone(),
            provider: self.config.provider.clone(),
            request_id,
        };

        if let Err(e) = self.api_sender.send(request) {
            eprintln!("[DEBUG] Failed to send request: {}", e);
            self.status = "Internal error: failed to send request".to_string();
            self.is_checking = false;
            self.current_request_id = None;
        }
    }

    fn process_api_responses(&mut self) {
        // Process all pending responses
        loop {
            match self.api_receiver.try_recv() {
                Ok(response) => {
                    let (request_id, is_success) = match &response {
                        ApiResponse::Success { request_id, .. } => (*request_id, true),
                        ApiResponse::Error { request_id, .. } => (*request_id, false),
                    };

                    // Check if this response is for the current request
                    if self.current_request_id != Some(request_id) {
                        eprintln!("[DEBUG] Discarding stale response #{} (current={})", 
                                  request_id, 
                                  self.current_request_id.map(|id| id.to_string()).unwrap_or("none".into()));
                        continue;
                    }

                    eprintln!("[DEBUG] Processing response #{}, success={}", request_id, is_success);
                    self.is_checking = false;
                    self.current_request_id = None;

                    match response {
                        ApiResponse::Success { suggestions, .. } => {
                            self.suggestions = suggestions;
                            if self.suggestions.is_empty() {
                                self.status = "All good!".to_string();
                            } else {
                                self.status = format!("{} suggestion(s)", self.suggestions.len());
                            }
                        }
                        ApiResponse::Error { message, .. } => {
                            self.status = message;
                        }
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    eprintln!("[DEBUG] API channel disconnected!");
                    self.status = "Internal error: API thread died".to_string();
                    break;
                }
            }
        }
    }

    fn apply_suggestion(&mut self, suggestion_id: &str) {
        let suggestion = self.suggestions.iter().find(|s| s.id == suggestion_id).cloned();
        
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

    fn save_settings(&mut self) {
        self.config.api_key = self.temp_api_key.trim().to_string();
        self.config.provider = self.temp_provider.clone();
        self.config.model = if self.temp_model.trim().is_empty() {
            self.config.provider.default_model().to_string()
        } else {
            self.temp_model.trim().to_string()
        };
        self.config.save();
        self.show_settings = false;
        self.status = "Settings saved".to_string();
        eprintln!("[DEBUG] Settings saved: provider={}, model={}", self.config.provider.name(), self.config.model);
    }
}

impl eframe::App for GrammyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process any pending API responses
        self.process_api_responses();

        // Debounced check - only trigger after user stops typing
        if let Some(edit_time) = self.last_edit_time {
            let elapsed = edit_time.elapsed();
            if elapsed >= Duration::from_millis(DEBOUNCE_MS) {
                eprintln!("[DEBUG] Debounce timer fired after {:?}", elapsed);
                self.last_edit_time = None;
                self.check_text();
            } else {
                // Keep polling until debounce expires
                ctx.request_repaint_after(Duration::from_millis(50));
            }
        }

        // Keep repainting while checking to receive responses
        if self.is_checking {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        // Settings modal
        if self.show_settings {
            egui::Window::new(egui::RichText::new("Settings").color(COL_TEXT).size(18.0).strong())
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .fixed_size([420.0, 320.0])
                .frame(egui::Frame::window(&ctx.style()).fill(COL_PANEL).rounding(12.0))
                .show(ctx, |ui| {
                    ui.add_space(8.0);
                    
                    // Provider selection
                    ui.label(egui::RichText::new("API Provider").color(COL_TEXT).size(13.0));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let openai_selected = self.temp_provider == ApiProvider::OpenAI;
                        let openrouter_selected = self.temp_provider == ApiProvider::OpenRouter;

                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("OpenAI")
                                        .color(if openai_selected { COL_BG } else { COL_TEXT })
                                        .size(13.0),
                                )
                                .fill(if openai_selected { COL_ACCENT } else { COL_EDITOR_BG })
                                .rounding(6.0)
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    if openai_selected {
                                        egui::Color32::TRANSPARENT
                                    } else {
                                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30)
                                    },
                                ))
                                .min_size(egui::vec2(96.0, 28.0)),
                            )
                            .clicked()
                        {
                            self.temp_provider = ApiProvider::OpenAI;
                            if self.temp_model.is_empty() || self.temp_model.starts_with("openai/") {
                                self.temp_model = ApiProvider::OpenAI.default_model().to_string();
                            }
                        }
                        ui.add_space(8.0);

                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("OpenRouter")
                                        .color(if openrouter_selected { COL_BG } else { COL_TEXT })
                                        .size(13.0),
                                )
                                .fill(if openrouter_selected { COL_ACCENT } else { COL_EDITOR_BG })
                                .rounding(6.0)
                                .stroke(egui::Stroke::new(
                                    1.0,
                                    if openrouter_selected {
                                        egui::Color32::TRANSPARENT
                                    } else {
                                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30)
                                    },
                                ))
                                .min_size(egui::vec2(120.0, 28.0)),
                            )
                            .clicked()
                        {
                            self.temp_provider = ApiProvider::OpenRouter;
                            if self.temp_model.is_empty() || !self.temp_model.contains('/') {
                                self.temp_model = ApiProvider::OpenRouter.default_model().to_string();
                            }
                        }
                    });
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("OpenRouter supports many models from different providers.").size(11.0).color(COL_MUTED));

                    ui.add_space(12.0);
                    
                    ui.label(egui::RichText::new("API Key").color(COL_TEXT).size(13.0));
                    ui.add_space(4.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.temp_api_key)
                            .password(true)
                            .hint_text(if self.temp_provider == ApiProvider::OpenAI { "sk-..." } else { "sk-or-..." })
                            .text_color(COL_TEXT)
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Your API key is stored locally and only sent to the selected provider.").size(11.0).color(COL_MUTED));

                    ui.add_space(12.0);

                    ui.label(egui::RichText::new("Model").color(COL_TEXT).size(13.0));
                    ui.add_space(4.0);
                    ui.add(
                        egui::TextEdit::singleline(&mut self.temp_model)
                            .hint_text(self.temp_provider.default_model())
                            .text_color(COL_TEXT)
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(4.0);
                    let model_hint = if self.temp_provider == ApiProvider::OpenAI {
                        "e.g., gpt-4o-mini, gpt-4o, gpt-4-turbo"
                    } else {
                        "e.g., openai/gpt-4o-mini, anthropic/claude-3.5-sonnet"
                    };
                    ui.label(egui::RichText::new(model_hint).size(11.0).color(COL_MUTED));

                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.add(egui::Button::new(egui::RichText::new("Save").color(COL_BG)).fill(COL_ACCENT).rounding(6.0)).clicked() {
                                self.save_settings();
                            }
                            ui.add_space(8.0);
                            if ui.add(egui::Button::new(egui::RichText::new("Cancel").color(COL_TEXT)).rounding(6.0)).clicked() {
                                self.show_settings = false;
                            }
                        });
                    });
                });
        }

        // Header panel with brand and settings button
        egui::TopBottomPanel::top("header")
            .exact_height(52.0)
            .frame(egui::Frame::none().fill(COL_PANEL).stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 15))))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(16.0);
                    ui.label(egui::RichText::new("Grammy").size(20.0).strong().color(COL_TEXT));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);
                        if ui.add(egui::Button::new(egui::RichText::new("⚙").size(18.0).color(COL_TEXT)).frame(false)).clicked() {
                            self.temp_api_key = self.config.api_key.clone();
                            self.temp_model = self.config.model.clone();
                            self.temp_provider = self.config.provider.clone();
                            self.show_settings = true;
                        }
                    });
                });
            });

        // Status bar at bottom
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(36.0)
            .frame(egui::Frame::none().fill(COL_PANEL).stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 15))))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(16.0);
                    let status_color = if self.status.contains("error") || self.status.contains("Error") {
                        COL_DANGER
                    } else if self.status == "All good!" {
                        COL_SUCCESS
                    } else {
                        COL_MUTED
                    };
                    ui.label(egui::RichText::new(&self.status).size(12.0).color(status_color));
                    ui.label(egui::RichText::new("·").size(12.0).color(COL_MUTED));
                    ui.label(egui::RichText::new("Suggestions appear as you type").size(12.0).color(COL_MUTED));
                });
            });

        // Right sidebar for suggestions
        egui::SidePanel::right("suggestions_panel")
            .min_width(300.0)
            .max_width(450.0)
            .default_width(360.0)
            .resizable(true)
            .frame(egui::Frame::none().fill(COL_PANEL).stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 15))))
            .show(ctx, |ui| {
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(egui::RichText::new("Suggestions").size(15.0).strong().color(COL_TEXT));
                });
                ui.add_space(12.0);
                
                // Separator
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    let rect = ui.available_rect_before_wrap();
                    ui.painter().hline(
                        rect.left()..=rect.right() - 16.0,
                        rect.top(),
                        egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20)),
                    );
                });
                ui.add_space(12.0);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        
                        if self.suggestions.is_empty() {
                            ui.add_space(40.0);
                            ui.vertical_centered(|ui| {
                                ui.label(egui::RichText::new("No suggestions").size(14.0).color(COL_MUTED));
                            });
                        } else {
                            let suggestions_clone: Vec<_> = self.suggestions.iter().cloned().collect();
                            let mut suggestion_to_apply = None;

                            ui.horizontal(|ui| {
                                ui.add_space(12.0);
                                ui.vertical(|ui| {
                                    ui.set_width(ui.available_width() - 24.0);
                                    
                                    for suggestion in &suggestions_clone {
                                        let is_hovered = self.hovered_suggestion.as_ref() == Some(&suggestion.id);
                                        
                                        let card_resp = egui::Frame::none()
                                            .fill(if is_hovered { egui::Color32::from_rgb(25, 38, 70) } else { COL_EDITOR_BG })
                                            .rounding(10.0)
                                            .inner_margin(14.0)
                                            .stroke(egui::Stroke::new(1.0, if is_hovered { COL_ACCENT } else { egui::Color32::from_rgba_unmultiplied(255, 255, 255, 25) }))
                                            .show(ui, |ui| {
                                                ui.set_width(ui.available_width());

                                                ui.label(egui::RichText::new(&suggestion.message).size(12.0).color(COL_MUTED));
                                                ui.add_space(8.0);

                                                ui.horizontal_wrapped(|ui| {
                                                    ui.label(egui::RichText::new(&suggestion.original).size(14.0).color(COL_DANGER).strikethrough());
                                                    ui.label(egui::RichText::new(" → ").size(13.0).color(COL_MUTED));
                                                    ui.label(egui::RichText::new(&suggestion.replacement).size(14.0).color(COL_SUCCESS).strong());
                                                });

                                                ui.add_space(10.0);

                                                if ui.add(egui::Button::new(egui::RichText::new("Accept").size(12.0).color(COL_BG)).fill(COL_SUCCESS).rounding(6.0).min_size(egui::vec2(70.0, 28.0))).clicked() {
                                                    suggestion_to_apply = Some(suggestion.id.clone());
                                                }
                                            });

                                        if card_resp.response.hovered() {
                                            self.hovered_suggestion = Some(suggestion.id.clone());
                                        }

                                        ui.add_space(10.0);
                                    }
                                });
                            });

                            if let Some(id) = suggestion_to_apply {
                                self.apply_suggestion(&id);
                            }
                        }
                    });
            });

        // Main editor panel - fills remaining space
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(COL_BG).inner_margin(egui::Margin::symmetric(20.0, 16.0)))
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Your text").size(12.0).color(COL_MUTED));
                ui.add_space(10.0);

                // Editor frame that fills available space
                let available = ui.available_size();
                
                egui::Frame::none()
                    .fill(COL_EDITOR_BG)
                    .rounding(12.0)
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(255, 255, 255, 25)))
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(available.x - 32.0, available.y - 60.0));
                        
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.set_min_width(ui.available_width());
                                
                                let response = render_highlighted_text(
                                    ui,
                                    &mut self.text,
                                    &self.suggestions,
                                    &mut self.hovered_suggestion,
                                );
                                
                                if response.changed() {
                                    self.suggestions.clear();
                                    self.schedule_check();
                                }
                            });
                    });
            });
    }
}

fn render_highlighted_text(
    ui: &mut egui::Ui,
    text: &mut String,
    suggestions: &[Suggestion],
    hovered_suggestion: &mut Option<String>,
) -> egui::Response {
    // Custom layouter for all cases to ensure proper text color
    let suggestion_ranges: Vec<_> = suggestions
        .iter()
        .map(|s| (s.offset, s.offset + s.length, s.id.clone()))
        .collect();

    let hovered_id = hovered_suggestion.clone();

    let mut layouter = move |ui: &egui::Ui, text_content: &str, wrap_width: f32| {
        let mut job = egui::text::LayoutJob::default();
        job.wrap.max_width = wrap_width;

        if suggestion_ranges.is_empty() {
            // No suggestions - just render all text in white
            job.append(
                text_content,
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::proportional(15.0),
                    color: COL_TEXT,
                    line_height: Some(24.0),
                    ..Default::default()
                },
            );
        } else {
            let mut pos = 0;

            for (start, end, ref id) in &suggestion_ranges {
                let start = *start;
                let end = (*end).min(text_content.len());

                // Text before this suggestion
                if start > pos && pos < text_content.len() {
                    let slice_end = start.min(text_content.len());
                    if let Some(slice) = text_content.get(pos..slice_end) {
                        job.append(
                            slice,
                            0.0,
                            egui::TextFormat {
                                font_id: egui::FontId::proportional(15.0),
                                color: COL_TEXT,
                                line_height: Some(24.0),
                                ..Default::default()
                            },
                        );
                    }
                }

                // The suggestion itself (highlighted)
                if start < text_content.len() && end <= text_content.len() {
                    if let Some(slice) = text_content.get(start..end) {
                        let is_hovered = hovered_id.as_ref() == Some(id);
                        job.append(
                            slice,
                            0.0,
                            egui::TextFormat {
                                font_id: egui::FontId::proportional(15.0),
                                color: COL_DANGER,
                                background: if is_hovered {
                                    egui::Color32::from_rgba_unmultiplied(255, 107, 107, 80)
                                } else {
                                    egui::Color32::from_rgba_unmultiplied(255, 107, 107, 40)
                                },
                                underline: egui::Stroke::new(2.0, COL_DANGER),
                                line_height: Some(24.0),
                                ..Default::default()
                            },
                        );
                    }
                }

                pos = end;
            }

            // Remaining text after last suggestion
            if pos < text_content.len() {
                if let Some(slice) = text_content.get(pos..) {
                    job.append(
                        slice,
                        0.0,
                        egui::TextFormat {
                            font_id: egui::FontId::proportional(15.0),
                            color: COL_TEXT,
                            line_height: Some(24.0),
                            ..Default::default()
                        },
                    );
                }
            }
        }

        ui.fonts(|f| f.layout_job(job))
    };

    ui.add(
        egui::TextEdit::multiline(text)
            .font(egui::FontId::proportional(15.0))
            .text_color(COL_TEXT)
            .desired_width(f32::INFINITY)
            .min_size(ui.available_size())
            .hint_text(egui::RichText::new("Paste or type here...").color(COL_MUTED).size(15.0))
            .frame(false)
            .margin(egui::Margin::ZERO)
            .layouter(&mut layouter),
    )
}

fn setup_custom_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();

    style.visuals.dark_mode = true;
    style.visuals.override_text_color = Some(COL_TEXT);
    style.visuals.panel_fill = COL_PANEL;
    style.visuals.window_fill = COL_PANEL;
    style.visuals.extreme_bg_color = COL_BG;
    style.visuals.faint_bg_color = COL_EDITOR_BG;
    
    let border = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30);
    
    style.visuals.widgets.noninteractive.bg_fill = COL_EDITOR_BG;
    style.visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, COL_TEXT);
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, border);
    style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(8.0);

    style.visuals.widgets.inactive.bg_fill = COL_EDITOR_BG;
    style.visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, COL_TEXT);
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, border);
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);

    style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgba_unmultiplied(110, 168, 254, 50);
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, COL_TEXT);
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, COL_ACCENT);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);

    style.visuals.widgets.active.bg_fill = egui::Color32::from_rgba_unmultiplied(110, 168, 254, 70);
    style.visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, COL_TEXT);
    style.visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, COL_ACCENT);
    style.visuals.widgets.active.rounding = egui::Rounding::same(8.0);

    style.visuals.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(110, 168, 254, 120);
    style.visuals.selection.stroke = egui::Stroke::new(1.0, COL_ACCENT);

    style.visuals.window_shadow = egui::epaint::Shadow {
        offset: [0.0, 8.0].into(),
        blur: 24.0,
        spread: 0.0,
        color: egui::Color32::from_rgba_unmultiplied(0, 0, 0, 120),
    };

    style.visuals.window_rounding = egui::Rounding::same(12.0);
    style.visuals.window_stroke = egui::Stroke::new(1.0, border);

    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(14.0, 8.0);
    style.spacing.window_margin = egui::Margin::same(18.0);

    // Larger default text
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::proportional(14.0),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(13.0),
    );

    ctx.set_style(style);
}
