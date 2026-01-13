use eframe::egui;
use egui::{Color32, FontId, Margin, Rounding, Stroke, Vec2};
use crate::config::ApiProvider;
use crate::suggestion::Severity;
use super::state::App;

const COL_BG: Color32 = Color32::from_rgb(245, 241, 234);
const COL_SURFACE: Color32 = Color32::from_rgb(255, 255, 255);
const COL_PANEL: Color32 = Color32::from_rgb(255, 255, 255);
const COL_PANEL_ALT: Color32 = Color32::from_rgb(250, 246, 240);
const COL_EDITOR_BG: Color32 = Color32::from_rgb(255, 255, 255);
const COL_TEXT: Color32 = Color32::from_rgb(28, 33, 37);
const COL_MUTED: Color32 = Color32::from_rgb(102, 108, 113);
const COL_DANGER: Color32 = Color32::from_rgb(196, 58, 58);
const COL_WARNING: Color32 = Color32::from_rgb(217, 130, 43);
const COL_SUGGESTION: Color32 = Color32::from_rgb(191, 142, 36);
const COL_SUCCESS: Color32 = Color32::from_rgb(20, 131, 98);
const COL_DANGER_BG: Color32 = Color32::from_rgb(255, 236, 236);
const COL_WARNING_BG: Color32 = Color32::from_rgb(255, 244, 231);
const COL_SUGGESTION_BG: Color32 = Color32::from_rgb(255, 248, 230);
const COL_ACCENT: Color32 = Color32::from_rgb(15, 109, 92);
const COL_ACCENT_SOFT: Color32 = Color32::from_rgb(217, 236, 231);
const COL_ACCENT_TEXT: Color32 = Color32::from_rgb(255, 255, 255);
const COL_HEADER: Color32 = Color32::from_rgb(252, 244, 230);
const COL_BORDER: Color32 = Color32::from_rgb(226, 217, 205);
const COL_DIVIDER: Color32 = Color32::from_rgb(232, 224, 213);
const CONTROL_HEIGHT: f32 = 34.0;
const CONTROL_GAP: f32 = 10.0;
const PANEL_RADIUS: f32 = 18.0;

pub fn show(ctx: &egui::Context, app: &mut App) {
    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(COL_BG)
                .inner_margin(Margin::symmetric(26.0, 16.0)),
        )
        .show(ctx, |ui| {
            ui.add_space(12.0);
            show_header(ui, app);
            ui.add_space(18.0);
            
            egui::ScrollArea::vertical()
                .id_salt("main_scroll")
                .show(ui, |ui| {
                    let is_wide = ui.available_width() >= 980.0;
                    let panel_height = ui.available_height();
                    
                    if is_wide {
                        ui.horizontal(|ui| {
                            let left_width = (ui.available_width() - 22.0) * 0.64;
                            ui.allocate_ui_with_layout(
                                Vec2::new(left_width, panel_height),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    show_editor_panel(ui, app);
                                },
                            );
                            ui.add_space(8.0);
                            vertical_separator(ui, panel_height);
                            ui.add_space(8.0);
                            ui.allocate_ui_with_layout(
                                Vec2::new(ui.available_width(), panel_height),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    show_suggestions_panel(ui, app);
                                },
                            );
                        });
                    } else {
                        show_editor_panel(ui, app);
                        ui.add_space(16.0);
                        show_suggestions_panel(ui, app);
                    }
                });
            
            ui.add_space(14.0);
            show_footer(ui, app);
        });
    
    if app.show_settings {
        show_settings_modal(ctx, app);
    }
}

fn show_header(ui: &mut egui::Ui, app: &mut App) {
    egui::Frame {
        inner_margin: Margin::symmetric(20.0, 16.0),
        outer_margin: Margin::ZERO,
        rounding: Rounding::same(PANEL_RADIUS),
        shadow: egui::epaint::Shadow::NONE,
        fill: COL_HEADER,
        stroke: Stroke::new(1.0, COL_BORDER),
    }
    .show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("Grammy")
                        .size(30.0)
                        .strong()
                        .color(COL_TEXT),
                );
                ui.label(
                    egui::RichText::new("Simple grammar checks for everyday drafts.")
                        .size(12.5)
                        .color(COL_MUTED),
                );
            });
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_sized(
                        [128.0, CONTROL_HEIGHT],
                        egui::Button::new(
                            egui::RichText::new("Settings").color(COL_ACCENT),
                        )
                        .fill(COL_ACCENT_SOFT)
                        .stroke(Stroke::new(1.0, COL_ACCENT)),
                    )
                    .clicked()
                {
                    app.show_settings = true;
                    app.temp_openai_api_key = app.config.openai_api_key.clone();
                    app.temp_openrouter_api_key = app.config.openrouter_api_key.clone();
                    app.temp_gemini_api_key = app.config.gemini_api_key.clone();
                    app.temp_model = app.config.model.clone();
                    app.temp_provider = app.config.provider.clone();
                    app.temp_debounce_ms = app.config.debounce_ms as f32;
                    app.show_api_key = false;
                    app.test_status.clear();
                    app.fetch_models_if_needed();
                }
                
                let status_label = if app.is_checking {
                    "Checking"
                } else if app.suggestions.is_empty() && !app.editor_text.trim().is_empty() {
                    "Clear"
                } else {
                    "Ready"
                };
                let (status_color, status_bg) = if status_label == "Clear" {
                    (COL_SUCCESS, Color32::from_rgb(244, 248, 246))
                } else if status_label == "Checking" {
                    (COL_WARNING, Color32::from_rgb(252, 246, 238))
                } else {
                    (COL_MUTED, Color32::from_rgb(242, 243, 245))
                };
                
                ui.add_space(12.0);
                egui::Frame::none()
                    .fill(status_bg)
                    .stroke(Stroke::new(1.0, COL_BORDER))
                    .rounding(Rounding::same(999.0))
                    .inner_margin(Margin::symmetric(10.0, 4.0))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(status_label)
                                .size(11.0)
                                .color(status_color),
                        );
                    });
            });
        });
    });
}

fn show_editor_panel(ui: &mut egui::Ui, app: &mut App) {
    egui::Frame {
        inner_margin: Margin::symmetric(20.0, 18.0),
        outer_margin: Margin::ZERO,
        rounding: Rounding::same(PANEL_RADIUS),
        shadow: egui::epaint::Shadow::NONE,
        fill: COL_PANEL,
        stroke: Stroke::new(1.0, COL_BORDER),
    }
    .show(ui, |ui| {
        let word_count = app.editor_text.split_whitespace().count();
        
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Draft")
                    .size(17.0)
                    .strong()
                    .color(COL_TEXT),
            );
            ui.label(
                egui::RichText::new(format!("{} words", word_count))
                    .size(12.0)
                    .color(COL_MUTED),
            );
        });
        
        ui.add_space(12.0);
        
        let old_text = app.editor_text.clone();
        let editor_height = ui.available_height().max(320.0);
        
        ui.add_sized(
            [ui.available_width(), editor_height],
            egui::TextEdit::multiline(&mut app.editor_text)
                .id_source(app.editor_id)
                .font(FontId::monospace(14.5))
                .desired_width(f32::INFINITY)
                .desired_rows(16)
                .frame(true)
                .text_color(COL_TEXT)
                .background_color(COL_EDITOR_BG),
        );
        
        let new_text = app.editor_text.clone();
        
        if old_text != new_text {
            app.suggestions.clear();
            app.hovered_suggestion_id = None;
            app.last_edit_time = Some(std::time::Instant::now());
            app.draft_dirty = true;
            if app.is_checking {
                app.pending_recheck = true;
            }
        }
    });
}

fn show_suggestions_panel(ui: &mut egui::Ui, app: &mut App) {
    egui::Frame {
        inner_margin: Margin::symmetric(20.0, 18.0),
        outer_margin: Margin::ZERO,
        rounding: Rounding::same(PANEL_RADIUS),
        shadow: egui::epaint::Shadow::NONE,
        fill: COL_PANEL_ALT,
        stroke: Stroke::new(1.0, COL_BORDER),
    }
    .show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("Suggestions")
                    .size(17.0)
                    .strong()
                    .color(COL_TEXT),
            );
            
            egui::Frame::none()
                .fill(COL_PANEL_ALT)
                .rounding(Rounding::same(999.0))
                .inner_margin(Margin::symmetric(10.0, 4.0))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(format!("{}", app.suggestions.len()))
                            .size(12.0)
                            .color(COL_MUTED),
                    );
                });
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let can_check = !app.is_checking && !app.editor_text.trim().is_empty();
                ui.add_enabled_ui(can_check, |ui| {
                    if ui
                        .add_sized(
                            [128.0, CONTROL_HEIGHT + 8.0],
                            egui::Button::new(
                                egui::RichText::new("Check now").color(COL_ACCENT_TEXT),
                            )
                            .fill(COL_ACCENT)
                            .stroke(Stroke::NONE),
                        )
                        .clicked()
                    {
                        app.last_checked_text.clear();
                        app.check_text();
                    }
                });
            });
        });
        
        ui.add_space(12.0);
        
        if app.editor_text.trim().is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(18.0);
                ui.label(
                    egui::RichText::new("Start typing to get suggestions.")
                        .size(13.0)
                        .color(COL_MUTED),
                );
                ui.add_space(18.0);
            });
        } else if app.last_edit_time.is_some() {
            ui.vertical_centered(|ui| {
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new("Waiting to check...")
                        .size(13.0)
                        .color(COL_MUTED),
                );
                ui.add_space(12.0);
            });
        } else if app.is_checking {
            ui.vertical_centered(|ui| {
                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new("Checking...")
                        .size(13.0)
                        .color(COL_MUTED),
                );
                ui.add_space(12.0);
            });
        } else if app.suggestions.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(18.0);
                ui.label(
                    egui::RichText::new("No issues found.")
                        .size(14.0)
                        .strong()
                        .color(COL_TEXT),
                );
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("Keep writing or check again after edits.")
                        .size(12.0)
                        .color(COL_MUTED),
                );
                ui.add_space(18.0);
            });
        } else {
            let suggestion_ids: Vec<String> = app.suggestions.iter().map(|s| s.id.clone()).collect();
            for suggestion_id in suggestion_ids {
                if let Some(suggestion) = app
                    .suggestions
                    .iter()
                    .find(|s| s.id == suggestion_id)
                    .cloned()
                {
                    show_suggestion_card(ui, app, &suggestion);
                    ui.add_space(12.0);
                }
            }
        }

        ui.add_space(ui.available_height());
    });
}

fn show_suggestion_card(
    ui: &mut egui::Ui,
    app: &mut App,
    suggestion: &crate::suggestion::Suggestion,
) {
    let (severity_label, severity_color, severity_bg) = match suggestion.severity {
        Severity::Error => ("Error", COL_DANGER, COL_DANGER_BG),
        Severity::Warning => ("Warning", COL_WARNING, COL_WARNING_BG),
        Severity::Suggestion => ("Suggestion", COL_SUGGESTION, COL_SUGGESTION_BG),
    };
    
    egui::Frame {
        inner_margin: Margin::symmetric(16.0, 14.0),
        outer_margin: Margin::ZERO,
        rounding: Rounding::same(14.0),
        shadow: egui::epaint::Shadow::NONE,
        fill: COL_SURFACE,
        stroke: Stroke::new(1.0, COL_BORDER),
    }
    .show(ui, |ui| {
        ui.horizontal(|ui| {
            egui::Frame::none()
                .fill(severity_bg)
                .rounding(Rounding::same(6.0))
                .inner_margin(Margin::symmetric(8.0, 4.0))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(severity_label)
                            .size(11.0)
                            .strong()
                            .color(severity_color),
                    );
                });
            
            ui.add(
                egui::Label::new(
                    egui::RichText::new(&suggestion.message)
                        .size(13.0)
                        .color(COL_MUTED),
                )
                .wrap(),
            );
        });
        
        ui.add_space(8.0);
        
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new(&suggestion.original)
                    .size(14.0)
                    .color(severity_color),
            );
            
            if let Some(ref replacement) = suggestion.replacement {
                ui.label(
                    egui::RichText::new(" -> ")
                        .size(14.0)
                        .color(COL_MUTED),
                );
                ui.label(
                    egui::RichText::new(replacement)
                        .size(14.0)
                        .color(COL_SUCCESS),
                );
            }
        });
        
        ui.add_space(10.0);
        
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let button_width = (ui.available_width() - 8.0) / 2.0;
                if ui
                    .add_sized(
                        [button_width, CONTROL_HEIGHT],
                        egui::Button::new(egui::RichText::new("Accept").color(COL_ACCENT_TEXT))
                            .fill(COL_ACCENT)
                            .stroke(Stroke::NONE),
                    )
                    .clicked()
                {
                    app.apply_suggestion(&suggestion.id);
                }
                
                ui.add_space(8.0);
                
                if ui
                    .add_sized(
                        [button_width, CONTROL_HEIGHT],
                        egui::Button::new("Dismiss")
                            .fill(COL_PANEL_ALT)
                            .stroke(Stroke::new(1.0, COL_BORDER)),
                    )
                    .clicked()
                {
                    app.suggestions.retain(|s| s.id != suggestion.id);
                    if app.hovered_suggestion_id.as_deref() == Some(suggestion.id.as_str()) {
                        app.hovered_suggestion_id = None;
                    }
                    
                    if !app.is_checking {
                        if app.suggestions.is_empty() {
                            app.status = "All good!".to_string();
                        } else {
                            app.status = format!("{} suggestion(s)", app.suggestions.len());
                        }
                    }
                }
            });
        });
    });
}

fn show_footer(ui: &mut egui::Ui, app: &mut App) {
    let status_color = if app.status.contains("error") || app.status.contains("Error") {
        COL_DANGER
    } else if app.status == "All good!" {
        COL_SUCCESS
    } else {
        COL_MUTED
    };
    
    egui::Frame {
        inner_margin: Margin::symmetric(16.0, 10.0),
        outer_margin: Margin::ZERO,
        rounding: Rounding::same(14.0),
        shadow: egui::epaint::Shadow::NONE,
        fill: COL_PANEL_ALT,
        stroke: Stroke::new(1.0, COL_BORDER),
    }
    .show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(&app.status)
                    .size(12.0)
                    .color(status_color),
            );
            
            ui.label(
                egui::RichText::new(" | ")
                    .size(12.0)
                    .color(COL_MUTED),
            );
            
            ui.label(
                egui::RichText::new("Auto-check runs after you pause typing.")
                    .size(12.0)
                    .color(COL_MUTED),
            );
        });
    });
}

fn show_settings_modal(ctx: &egui::Context, app: &mut App) {
    egui::Window::new("Settings")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .fixed_size(Vec2::new(520.0, 560.0))
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Frame::none()
                    .fill(COL_ACCENT_SOFT)
                    .rounding(Rounding::same(14.0))
                    .stroke(Stroke::new(1.0, COL_BORDER))
                    .inner_margin(Margin::symmetric(18.0, 12.0))
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new("Settings")
                                .size(16.0)
                                .strong()
                                .color(COL_ACCENT),
                        );
                        ui.label(
                            egui::RichText::new("Manage providers and auto-check behavior.")
                                .size(12.0)
                                .color(COL_MUTED),
                        );
                    });
                ui.add_space(14.0);
            
            ui.label(
                egui::RichText::new("API Provider")
                    .size(14.0)
                    .color(COL_TEXT),
            );
            ui.add_space(8.0);
            
            let providers = [ApiProvider::OpenAI, ApiProvider::OpenRouter, ApiProvider::Gemini];
            let button_width =
                ((ui.available_width() - (CONTROL_GAP * 2.0)) / 3.0).max(96.0);
            ui.horizontal(|ui| {
                for (index, provider) in providers.iter().enumerate() {
                let is_selected = app.temp_provider == *provider;
                let text_color = if is_selected { COL_ACCENT_TEXT } else { COL_TEXT };
                if ui
                    .add_sized(
                        Vec2::new(button_width, CONTROL_HEIGHT),
                        egui::Button::new(
                            egui::RichText::new(provider.name())
                                .size(13.0)
                                .color(text_color),
                        )
                        .rounding(Rounding::same(10.0))
                        .fill(if is_selected { COL_ACCENT } else { COL_PANEL_ALT })
                        .stroke(if is_selected {
                            Stroke::NONE
                        } else {
                            Stroke::new(1.0, COL_BORDER)
                        }),
                        )
                        .clicked()
                    {
                        app.temp_provider = provider.clone();
                        app.temp_model = provider.default_model().to_string();
                        app.test_status.clear();
                        app.show_api_key = false;
                        app.fetch_models_if_needed();
                    }
                    
                    if index < providers.len() - 1 {
                        ui.add_space(CONTROL_GAP);
                    }
                }
            });
            
            ui.add_space(16.0);
            
            ui.label(
                egui::RichText::new("API Key")
                    .size(14.0)
                    .color(COL_TEXT),
            );
            ui.add_space(8.0);
            
            ui.horizontal(|ui| {
                let placeholder = match app.temp_provider {
                    ApiProvider::OpenAI => "sk-...",
                    ApiProvider::OpenRouter => "sk-or-...",
                    ApiProvider::Gemini => "AIza...",
                };
                
                let api_key = match app.temp_provider {
                    ApiProvider::OpenAI => &mut app.temp_openai_api_key,
                    ApiProvider::OpenRouter => &mut app.temp_openrouter_api_key,
                    ApiProvider::Gemini => &mut app.temp_gemini_api_key,
                };
                
                let field_width = (ui.available_width() - 72.0).max(0.0);
                ui.add_sized(
                    Vec2::new(field_width, CONTROL_HEIGHT),
                    egui::TextEdit::singleline(api_key)
                        .password(!app.show_api_key)
                        .hint_text(egui::RichText::new(placeholder).color(COL_MUTED)),
                );
                
                if ui
                    .add_sized(
                        Vec2::new(64.0, CONTROL_HEIGHT),
                        egui::Button::new(if app.show_api_key { "Hide" } else { "Show" })
                            .rounding(Rounding::same(10.0))
                            .fill(COL_PANEL_ALT)
                            .stroke(Stroke::new(1.0, COL_BORDER)),
                    )
                    .clicked()
                {
                    app.show_api_key = !app.show_api_key;
                }
            });
            
            ui.add_space(16.0);
            
            ui.label(
                egui::RichText::new("Model")
                    .size(14.0)
                    .color(COL_TEXT),
            );
            ui.add_space(8.0);
            
            let models = match app.temp_provider {
                ApiProvider::OpenAI => &app.openai_models,
                ApiProvider::OpenRouter => &app.openrouter_models,
                ApiProvider::Gemini => &app.gemini_models,
            };
            
            if !models.is_empty() {
                egui::Frame::none()
                    .fill(COL_PANEL_ALT)
                    .stroke(Stroke::new(1.0, COL_BORDER))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(Margin::symmetric(8.0, 6.0))
                    .show(ui, |ui| {
                        egui::ComboBox::from_id_salt("model_combo")
                            .selected_text(app.temp_model.clone())
                            .width(ui.available_width())
                            .show_ui(ui, |ui| {
                                for model in models {
                                    if ui
                                        .selectable_value(&mut app.temp_model, model.clone(), model)
                                        .changed()
                                    {
                                        // Model changed
                                    }
                                }
                            });
                    });
            } else {
                ui.add_sized(
                    Vec2::new(ui.available_width(), CONTROL_HEIGHT),
                    egui::TextEdit::singleline(&mut app.temp_model)
                        .hint_text(egui::RichText::new("Enter model name...").color(COL_MUTED)),
                );
            }
            
            ui.add_space(16.0);
            
            ui.label(
                egui::RichText::new("Auto-check Delay")
                    .size(14.0)
                    .color(COL_TEXT),
            );
            ui.add_space(8.0);
            
            egui::Frame::none()
                .fill(COL_PANEL_ALT)
                .stroke(Stroke::new(1.0, COL_BORDER))
                .rounding(Rounding::same(10.0))
                .inner_margin(Margin::symmetric(10.0, 8.0))
                .show(ui, |ui| {
                    ui.add_sized(
                        Vec2::new(ui.available_width(), CONTROL_HEIGHT + 2.0),
                        egui::Slider::new(&mut app.temp_debounce_ms, 250.0..=5500.0)
                            .step_by(250.0)
                            .show_value(false),
                    );
                    
                    let debounce_text = if app.temp_debounce_ms > 5000.0 {
                        "Never".to_string()
                    } else {
                        format!("{:.1}s", app.temp_debounce_ms / 1000.0)
                    };
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(debounce_text)
                                .size(12.0)
                                .color(COL_MUTED),
                        );
                    });
                });
            
            ui.add_space(16.0);
            
            if ui
                .add_sized(
                    Vec2::new(ui.available_width(), 38.0),
                    egui::Button::new(if app.is_testing {
                        "Testing..."
                    } else {
                        "Test connection"
                    })
                    .rounding(Rounding::same(10.0))
                    .fill(COL_PANEL_ALT)
                    .stroke(Stroke::new(1.0, COL_BORDER)),
                )
                .clicked()
            {
                app.test_connection();
            }
            
            if !app.test_status.is_empty() {
                ui.add_space(6.0);
                let test_color = if app.test_status.contains("OK") {
                    COL_SUCCESS
                } else {
                    COL_DANGER
                };
                ui.label(
                    egui::RichText::new(&app.test_status)
                        .size(12.0)
                        .color(test_color),
                );
            }
            
            ui.add_space(20.0);
            
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_sized(
                            Vec2::new(140.0, CONTROL_HEIGHT),
                            egui::Button::new(
                                egui::RichText::new("Save Settings").color(COL_ACCENT_TEXT),
                            )
                                .rounding(Rounding::same(10.0))
                                .fill(COL_ACCENT)
                                .stroke(Stroke::new(1.0, COL_ACCENT)),
                        )
                        .clicked()
                    {
                        app.save_settings();
                    }
                    
                    ui.add_space(8.0);
                    
                    if ui
                        .add_sized(
                            Vec2::new(110.0, CONTROL_HEIGHT),
                            egui::Button::new("Cancel")
                                .rounding(Rounding::same(10.0))
                                .fill(COL_PANEL_ALT)
                                .stroke(Stroke::new(1.0, COL_BORDER)),
                        )
                        .clicked()
                    {
                        app.show_settings = false;
                    }
                });
            });
            
                ui.add_space(8.0);
            });
        });
}

fn vertical_separator(ui: &mut egui::Ui, height: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(1.0, height), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 0.0, COL_DIVIDER);
}
