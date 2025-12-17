use crate::suggestion::Suggestion;
use eframe::egui;

use super::style::{COL_DANGER, COL_MUTED, COL_TEXT};

pub(super) fn render_highlighted_text(
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
