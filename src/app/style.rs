use iced::widget::{button, container, rule};
use iced::{color, Background, Border, Color, Shadow, Theme, Vector};

// --- Palette ---
pub(super) const COL_BG: Color = color!(0x050510); // Very deep navy/black
pub(super) const COL_PANEL: Color = color!(0x1A1F35); // Lighter navy for panels (base)
pub(super) const COL_TEXT: Color = color!(0xE0E6ED); // Soft white
pub(super) const COL_MUTED: Color = color!(0x94A3B8); // Muted slate
pub(super) const COL_ACCENT: Color = color!(0x6366F1); // Indigo
pub(super) const COL_SUCCESS: Color = color!(0x10B981); // Emerald
pub(super) const COL_DANGER: Color = color!(0xEF4444); // Red - Errors
pub(super) const COL_WARNING: Color = color!(0xF59E0B); // Amber - Warnings
pub(super) const COL_SUGGESTION: Color = color!(0xFDE047); // Yellow - Suggestions
pub(super) const COL_BORDER: Color = Color {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 0.08,
};

// --- Theme ---
pub(super) fn theme(_state: &super::state::State) -> Theme {
    Theme::Dark
}

// --- Gradients ---
fn gradient_primary() -> Background {
    Background::Gradient(
        iced::gradient::Linear::new(iced::Radians(0.6))
            .add_stop(0.0, color!(0x4F46E5))
            .add_stop(1.0, color!(0x9333EA))
            .into(),
    )
}

fn gradient_primary_hover() -> Background {
    Background::Gradient(
        iced::gradient::Linear::new(iced::Radians(0.6))
            .add_stop(0.0, color!(0x6366F1))
            .add_stop(1.0, color!(0xA855F7))
            .into(),
    )
}

// --- Styles ---

pub(super) fn btn_primary(_theme: &Theme, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => Some(gradient_primary_hover()),
        _ => Some(gradient_primary()),
    };

    button::Style {
        background,
        text_color: Color::WHITE,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color {
                a: 0.5,
                ..COL_ACCENT
            },
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        snap: true,
    }
}

pub(super) fn btn_secondary(_theme: &Theme, status: button::Status) -> button::Style {
    let bg_alpha = match status {
        button::Status::Hovered => 0.15,
        button::Status::Pressed => 0.20,
        _ => 0.08,
    };

    button::Style {
        background: Some(Background::Color(Color {
            a: bg_alpha,
            ..Color::WHITE
        })),
        text_color: COL_TEXT,
        border: Border {
            color: Color {
                a: 0.1,
                ..Color::WHITE
            },
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub(super) fn btn_success(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, shadow) = match status {
        button::Status::Hovered => (
            Color {
                a: 1.0,
                ..COL_SUCCESS
            },
            Shadow {
                color: Color {
                    a: 0.4,
                    ..COL_SUCCESS
                },
                blur_radius: 12.0,
                offset: Vector::new(0.0, 2.0),
            },
        ),
        _ => (
            Color {
                a: 0.9,
                ..COL_SUCCESS
            },
            Shadow {
                color: Color {
                    a: 0.2,
                    ..COL_SUCCESS
                },
                blur_radius: 8.0,
                offset: Vector::new(0.0, 2.0),
            },
        ),
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: COL_BG,
        border: Border {
            radius: 8.0.into(),
            ..Border::default()
        },
        shadow,
        snap: true,
    }
}

pub(super) fn btn_ghost(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color {
            a: 0.1,
            ..Color::WHITE
        },
        button::Status::Pressed => Color {
            a: 0.15,
            ..Color::WHITE
        },
        _ => Color::TRANSPARENT,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            COL_TEXT
        } else {
            COL_MUTED
        },
        border: Border::default(),
        shadow: Shadow::default(),
        snap: true,
    }
}

pub(super) fn glass_container(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(COL_TEXT),
        background: Some(Background::Color(Color {
            a: 0.6,
            ..COL_PANEL
        })),
        border: Border {
            color: COL_BORDER,
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: Color {
                a: 0.3,
                ..Color::BLACK
            },
            offset: Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
        snap: true,
    }
}

pub(super) fn glass_editor(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(COL_TEXT),
        background: Some(Background::Color(Color {
            a: 0.4,
            ..color!(0x000000)
        })),
        border: Border {
            color: COL_BORDER,
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub(super) fn rule_muted(_theme: &Theme) -> rule::Style {
    rule::Style {
        color: Color { a: 0.1, ..COL_TEXT },
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    }
}

pub(super) fn text_input(
    _theme: &Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    let active = iced::widget::text_input::Style {
        background: Background::Color(Color {
            a: 0.2,
            ..COL_PANEL
        }),
        border: Border {
            color: COL_BORDER,
            width: 1.0,
            radius: 8.0.into(),
        },
        icon: COL_MUTED,
        placeholder: Color { a: 0.4, ..COL_TEXT },
        value: COL_TEXT,
        selection: Color {
            a: 0.2,
            ..COL_ACCENT
        },
    };

    match status {
        iced::widget::text_input::Status::Active => active,
        iced::widget::text_input::Status::Hovered => iced::widget::text_input::Style {
            border: Border {
                color: Color { a: 0.3, ..COL_TEXT },
                ..active.border
            },
            ..active
        },
        iced::widget::text_input::Status::Focused { .. } => iced::widget::text_input::Style {
            border: Border {
                color: COL_ACCENT,
                ..active.border
            },
            background: Background::Color(Color {
                a: 0.3,
                ..COL_PANEL
            }),
            ..active
        },
        iced::widget::text_input::Status::Disabled => iced::widget::text_input::Style {
            background: Background::Color(Color {
                a: 0.1,
                ..COL_PANEL
            }),
            value: COL_MUTED,
            ..active
        },
    }
}

pub(super) fn editor_style(
    _theme: &Theme,
    _status: iced::widget::text_editor::Status,
) -> iced::widget::text_editor::Style {
    iced::widget::text_editor::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        value: COL_TEXT,
        selection: Color {
            a: 0.2,
            ..COL_ACCENT
        },
        placeholder: Color { a: 0.4, ..COL_TEXT },
    }
}
