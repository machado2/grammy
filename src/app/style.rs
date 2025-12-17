use eframe::egui;

// Color palette
pub(super) const COL_BG: egui::Color32 = egui::Color32::from_rgb(11, 16, 32);
pub(super) const COL_PANEL: egui::Color32 = egui::Color32::from_rgb(18, 26, 50);
pub(super) const COL_EDITOR_BG: egui::Color32 = egui::Color32::from_rgb(15, 23, 48);
pub(super) const COL_TEXT: egui::Color32 = egui::Color32::from_rgb(232, 236, 255);
pub(super) const COL_MUTED: egui::Color32 = egui::Color32::from_rgb(169, 178, 211);
pub(super) const COL_ACCENT: egui::Color32 = egui::Color32::from_rgb(110, 168, 254);
pub(super) const COL_SUCCESS: egui::Color32 = egui::Color32::from_rgb(126, 231, 135);
pub(super) const COL_DANGER: egui::Color32 = egui::Color32::from_rgb(255, 107, 107);

pub(super) fn setup_custom_style(ctx: &egui::Context) {
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

    style.visuals.widgets.hovered.bg_fill =
        egui::Color32::from_rgba_unmultiplied(110, 168, 254, 50);
    style.visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, COL_TEXT);
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, COL_ACCENT);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);

    style.visuals.widgets.active.bg_fill =
        egui::Color32::from_rgba_unmultiplied(110, 168, 254, 70);
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
