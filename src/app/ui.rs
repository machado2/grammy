use eframe::egui;
use std::time::Duration;

use crate::config::ApiProvider;

use super::state::{GrammyApp, DEBOUNCE_MS};
use super::style::*;
use super::text_render::render_highlighted_text;

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
                            self.test_status.clear();
                            self.show_api_key = false;
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
                            self.test_status.clear();
                            self.show_api_key = false;
                        }
                    });
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("OpenRouter supports many models from different providers.")
                            .size(11.0)
                            .color(COL_MUTED),
                    );

                    ui.add_space(12.0);

                    ui.label(egui::RichText::new("API Key").color(COL_TEXT).size(13.0));
                    ui.add_space(4.0);
                    egui::Frame::none()
                        .fill(COL_EDITOR_BG)
                        .rounding(6.0)
                        .stroke(egui::Stroke::new(
                            1.0,
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
                        ))
                        .inner_margin(egui::Margin::symmetric(10.0, 7.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let key_ref: &mut String = if self.temp_provider == ApiProvider::OpenAI {
                                    &mut self.temp_openai_api_key
                                } else {
                                    &mut self.temp_openrouter_api_key
                                };

                                let eye = if self.show_api_key { "🙈" } else { "👁" };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new(eye).size(16.0).color(COL_TEXT),
                                        )
                                        .frame(false),
                                    )
                                    .clicked()
                                {
                                    self.show_api_key = !self.show_api_key;
                                }

                                ui.add(
                                    egui::TextEdit::singleline(key_ref)
                                        .password(!self.show_api_key)
                                        .hint_text(
                                            egui::RichText::new(if self.temp_provider == ApiProvider::OpenAI {
                                                "sk-..."
                                            } else {
                                                "sk-or-..."
                                            })
                                            .color(COL_MUTED),
                                        )
                                        .text_color(COL_TEXT)
                                        .desired_width(f32::INFINITY)
                                        .frame(false),
                                );
                            });
                        });
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "Your API key is stored locally and only sent to the selected provider.",
                        )
                        .size(11.0)
                        .color(COL_MUTED),
                    );

                    ui.add_space(12.0);

                    ui.label(egui::RichText::new("Model").color(COL_TEXT).size(13.0));
                    ui.add_space(4.0);
                    egui::Frame::none()
                        .fill(COL_EDITOR_BG)
                        .rounding(6.0)
                        .stroke(egui::Stroke::new(
                            1.0,
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
                        ))
                        .inner_margin(egui::Margin::symmetric(10.0, 7.0))
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.temp_model)
                                    .hint_text(
                                        egui::RichText::new(self.temp_provider.default_model())
                                            .color(COL_MUTED),
                                    )
                                    .text_color(COL_TEXT)
                                    .desired_width(f32::INFINITY)
                                    .frame(false),
                            );
                        });
                    ui.add_space(4.0);
                    let model_hint = if self.temp_provider == ApiProvider::OpenAI {
                        "e.g., gpt-4o-mini, gpt-4o, gpt-4-turbo"
                    } else {
                        "e.g., openai/gpt-4o-mini, anthropic/claude-3.5-sonnet"
                    };
                    ui.label(egui::RichText::new(model_hint).size(11.0).color(COL_MUTED));

                    ui.add_space(16.0);

                    ui.horizontal(|ui| {
                        let btn = egui::Button::new(
                            egui::RichText::new(if self.is_testing {
                                "Testing..."
                            } else {
                                "Test connection"
                            })
                            .color(if self.is_testing { COL_MUTED } else { COL_TEXT }),
                        )
                        .fill(COL_EDITOR_BG)
                        .rounding(6.0)
                        .stroke(egui::Stroke::new(
                            1.0,
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30),
                        ));

                        if ui.add_enabled(!self.is_testing, btn).clicked() {
                            self.start_test_connection();
                        }

                        if !self.test_status.is_empty() {
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(&self.test_status)
                                    .color(self.test_status_color)
                                    .size(12.0),
                            );
                        }
                    });

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new("Save").color(COL_BG),
                                    )
                                    .fill(COL_ACCENT)
                                    .rounding(6.0),
                                )
                                .clicked()
                            {
                                self.save_settings();
                            }
                            ui.add_space(8.0);
                            if ui
                                .add(
                                    egui::Button::new(egui::RichText::new("Cancel").color(COL_TEXT))
                                        .fill(COL_EDITOR_BG)
                                        .stroke(egui::Stroke::new(
                                            1.0,
                                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 35),
                                        ))
                                        .rounding(6.0),
                                )
                                .clicked()
                            {
                                self.show_settings = false;
                            }
                        });
                    });
                });
        }

        // Header panel with brand and settings button
        egui::TopBottomPanel::top("header")
            .exact_height(52.0)
            .frame(
                egui::Frame::none().fill(COL_PANEL).stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 15),
                )),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new("Grammy")
                            .size(20.0)
                            .strong()
                            .color(COL_TEXT),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(16.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("⚙").size(18.0).color(COL_TEXT),
                                )
                                .frame(false),
                            )
                            .clicked()
                        {
                            self.temp_openai_api_key = self.config.openai_api_key.clone();
                            self.temp_openrouter_api_key = self.config.openrouter_api_key.clone();
                            self.temp_model = self.config.model.clone();
                            self.temp_provider = self.config.provider.clone();
                            self.show_api_key = false;
                            self.test_status.clear();
                            self.test_status_color = COL_MUTED;
                            self.show_settings = true;
                        }
                    });
                });
            });

        // Status bar at bottom
        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(36.0)
            .frame(
                egui::Frame::none().fill(COL_PANEL).stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 15),
                )),
            )
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
                    ui.label(
                        egui::RichText::new("Suggestions appear as you type")
                            .size(12.0)
                            .color(COL_MUTED),
                    );
                });
            });

        // Right sidebar for suggestions
        egui::SidePanel::right("suggestions_panel")
            .min_width(300.0)
            .max_width(450.0)
            .default_width(360.0)
            .resizable(true)
            .frame(
                egui::Frame::none().fill(COL_PANEL).stroke(egui::Stroke::new(
                    1.0,
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 15),
                )),
            )
            .show(ctx, |ui| {
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new("Suggestions")
                            .size(15.0)
                            .strong()
                            .color(COL_TEXT),
                    );
                });
                ui.add_space(12.0);

                // Separator
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    let rect = ui.available_rect_before_wrap();
                    ui.painter().hline(
                        rect.left()..=rect.right() - 16.0,
                        rect.top(),
                        egui::Stroke::new(
                            1.0,
                            egui::Color32::from_rgba_unmultiplied(255, 255, 255, 20),
                        ),
                    );
                });
                ui.add_space(12.0);

                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.set_min_width(ui.available_width());

                    if self.suggestions.is_empty() {
                        ui.add_space(40.0);
                        ui.vertical_centered(|ui| {
                            ui.label(
                                egui::RichText::new("No suggestions")
                                    .size(14.0)
                                    .color(COL_MUTED),
                            );
                        });
                    } else {
                        let suggestions_clone: Vec<_> = self.suggestions.iter().cloned().collect();
                        let mut suggestion_to_apply = None;

                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.vertical(|ui| {
                                ui.set_width(ui.available_width() - 24.0);

                                for suggestion in &suggestions_clone {
                                    let is_hovered =
                                        self.hovered_suggestion.as_ref() == Some(&suggestion.id);

                                    let card_resp = egui::Frame::none()
                                        .fill(if is_hovered {
                                            egui::Color32::from_rgb(25, 38, 70)
                                        } else {
                                            COL_EDITOR_BG
                                        })
                                        .rounding(10.0)
                                        .inner_margin(14.0)
                                        .stroke(egui::Stroke::new(
                                            1.0,
                                            if is_hovered {
                                                COL_ACCENT
                                            } else {
                                                egui::Color32::from_rgba_unmultiplied(
                                                    255, 255, 255, 25,
                                                )
                                            },
                                        ))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());

                                            ui.label(
                                                egui::RichText::new(&suggestion.message)
                                                    .size(12.0)
                                                    .color(COL_MUTED),
                                            );
                                            ui.add_space(8.0);

                                            ui.horizontal_wrapped(|ui| {
                                                ui.label(
                                                    egui::RichText::new(&suggestion.original)
                                                        .size(14.0)
                                                        .color(COL_DANGER)
                                                        .strikethrough(),
                                                );
                                                ui.label(
                                                    egui::RichText::new(" → ")
                                                        .size(13.0)
                                                        .color(COL_MUTED),
                                                );
                                                ui.label(
                                                    egui::RichText::new(&suggestion.replacement)
                                                        .size(14.0)
                                                        .color(COL_SUCCESS)
                                                        .strong(),
                                                );
                                            });

                                            ui.add_space(10.0);

                                            if ui
                                                .add(
                                                    egui::Button::new(
                                                        egui::RichText::new("Accept")
                                                            .size(12.0)
                                                            .color(COL_BG),
                                                    )
                                                    .fill(COL_SUCCESS)
                                                    .rounding(6.0)
                                                    .min_size(egui::vec2(70.0, 28.0)),
                                                )
                                                .clicked()
                                            {
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
            .frame(
                egui::Frame::none()
                    .fill(COL_BG)
                    .inner_margin(egui::Margin::symmetric(20.0, 16.0)),
            )
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("Your text").size(12.0).color(COL_MUTED));
                ui.add_space(10.0);

                // Editor frame that fills available space
                let available = ui.available_size();

                egui::Frame::none()
                    .fill(COL_EDITOR_BG)
                    .rounding(12.0)
                    .stroke(egui::Stroke::new(
                        1.0,
                        egui::Color32::from_rgba_unmultiplied(255, 255, 255, 25),
                    ))
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(available.x - 32.0, available.y - 60.0));

                        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
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
