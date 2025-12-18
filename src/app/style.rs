use iced::{color, Theme};
use iced::widget::button;

pub(super) const COL_BG: iced::Color = color!(0x0B1020);
pub(super) const COL_PANEL: iced::Color = color!(0x121A32);
pub(super) const COL_EDITOR_BG: iced::Color = color!(0x0F1730);
pub(super) const COL_TEXT: iced::Color = color!(0xE8ECFF);
pub(super) const COL_MUTED: iced::Color = color!(0xA9B2D3);
pub(super) const COL_ACCENT: iced::Color = color!(0x6EA8FE);
pub(super) const COL_SUCCESS: iced::Color = color!(0x7EE787);
pub(super) const COL_DANGER: iced::Color = color!(0xFF6B6B);
pub(super) const COL_BUTTON_BG: iced::Color = color!(0x1E2A4A);

pub(super) fn theme(_state: &super::state::State) -> Theme {
    Theme::TokyoNight
}

pub(super) fn btn_secondary(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => iced::Color { a: 1.0, ..COL_ACCENT },
        button::Status::Pressed => iced::Color { a: 0.8, ..COL_ACCENT },
        _ => COL_BUTTON_BG,
    };
    let text_color = match status {
        button::Status::Hovered | button::Status::Pressed => COL_BG,
        _ => COL_TEXT,
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color,
        border: iced::Border {
            color: iced::Color { a: 0.3, ..COL_ACCENT },
            width: 1.0,
            radius: 6.0.into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

pub(super) fn btn_primary(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => iced::Color { r: 0.5, g: 0.75, b: 1.0, a: 1.0 },
        button::Status::Pressed => iced::Color { r: 0.4, g: 0.6, b: 0.9, a: 1.0 },
        _ => COL_ACCENT,
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: COL_BG,
        border: iced::Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

pub(super) fn btn_success(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => iced::Color { r: 0.6, g: 1.0, b: 0.6, a: 1.0 },
        button::Status::Pressed => iced::Color { r: 0.4, g: 0.8, b: 0.4, a: 1.0 },
        _ => COL_SUCCESS,
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: COL_BG,
        border: iced::Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

pub(super) fn btn_ghost(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => iced::Color { a: 0.15, ..COL_TEXT },
        button::Status::Pressed => iced::Color { a: 0.25, ..COL_TEXT },
        _ => iced::Color::TRANSPARENT,
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: COL_TEXT,
        border: iced::Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        shadow: iced::Shadow::default(),
        snap: true,
    }
}
