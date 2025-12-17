use iced::{color, theme, Theme};

pub(super) const COL_BG: iced::Color = color!(0x0B1020);
pub(super) const COL_PANEL: iced::Color = color!(0x121A32);
pub(super) const COL_EDITOR_BG: iced::Color = color!(0x0F1730);
pub(super) const COL_TEXT: iced::Color = color!(0xE8ECFF);
pub(super) const COL_MUTED: iced::Color = color!(0xA9B2D3);
pub(super) const COL_ACCENT: iced::Color = color!(0x6EA8FE);
pub(super) const COL_SUCCESS: iced::Color = color!(0x7EE787);
pub(super) const COL_DANGER: iced::Color = color!(0xFF6B6B);

pub(super) fn theme(_state: &super::state::State) -> Theme {
    Theme::TokyoNight
}

pub(super) fn background_style(_theme: &Theme) -> theme::Style {
    theme::Style {
        background_color: COL_BG,
        text_color: COL_TEXT,
    }
}
