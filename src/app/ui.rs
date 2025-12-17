use iced::widget::{
    button, column, container, row, rule, scrollable, text, text_editor, text_input, Column,
};
use iced::{Alignment, Element, Fill, Length, Padding, Theme};

use crate::config::ApiProvider;

use super::state::{Message, State};
use super::style::{COL_ACCENT, COL_BG, COL_DANGER, COL_EDITOR_BG, COL_MUTED, COL_PANEL, COL_SUCCESS, COL_TEXT};

pub(super) fn view(state: &State) -> Element<'_, Message> {
    let header = container(
        row![
            text("Grammy").size(22).style(|_t| iced::widget::text::Style {
                color: Some(COL_TEXT),
            }),
            iced::widget::Space::new().width(Fill),
            button(text("⚙").size(18))
                .on_press(Message::OpenSettings)
                .style(|theme: &Theme, status| {
                    let palette = theme.extended_palette();
                    match status {
                        iced::widget::button::Status::Hovered => {
                            iced::widget::button::Style::default()
                                .with_background(palette.primary.weak.color)
                        }
                        _ => iced::widget::button::text(theme, status),
                    }
                }),
        ]
        .align_y(Alignment::Center)
        .spacing(10)
        .padding(Padding::new(12.0)),
    )
    .width(Fill)
    .style(|_theme| container_bg(COL_PANEL, 0.0));

    let status_color = if state.status.contains("error") || state.status.contains("Error") {
        COL_DANGER
    } else if state.status == "All good!" {
        COL_SUCCESS
    } else {
        COL_MUTED
    };

    let status_bar = container(
        row![
            text(&state.status).size(12).style(move |_t| iced::widget::text::Style {
                color: Some(status_color),
            }),
            text(" · ").size(12).style(|_t| iced::widget::text::Style {
                color: Some(COL_MUTED),
            }),
            text("Suggestions appear as you type")
                .size(12)
                .style(|_t| iced::widget::text::Style {
                    color: Some(COL_MUTED),
                }),
        ]
        .align_y(Alignment::Center)
        .spacing(4)
        .padding(Padding::new(10.0)),
    )
    .width(Fill)
    .style(|_theme| container_bg(COL_PANEL, 0.0));

    let suggestions_panel = suggestions_sidebar(state);
    let editor_panel = editor(state);

    let main = row![
        editor_panel,
        suggestions_panel,
    ]
    .height(Fill)
    .width(Fill);

    let root = column![header, main, status_bar]
        .width(Fill)
        .height(Fill)
        .spacing(0)
        .align_x(Alignment::Start);

    let base = container(root)
        .width(Fill)
        .height(Fill)
        .style(|_theme| container_bg(COL_BG, 0.0));

    if state.show_settings {
        settings_modal(base.into(), state)
    } else {
        base.into()
    }
}

fn editor(state: &State) -> Element<'_, Message> {
    let title = text("Your text")
        .size(12)
        .style(|_t| iced::widget::text::Style {
            color: Some(COL_MUTED),
        });

    let editor = text_editor(&state.editor)
        .placeholder("Paste or type here...")
        .on_action(Message::EditorAction)
        .height(Fill)
        .padding(14)
        .size(15);

    let frame = container(editor)
        .width(Fill)
        .height(Fill)
        .padding(Padding::new(16.0))
        .style(|_theme| container_panel(COL_EDITOR_BG));

    container(column![title, frame].spacing(10))
        .width(Length::FillPortion(3))
        .height(Fill)
        .padding(Padding::new(16.0))
        .into()
}

fn suggestions_sidebar(state: &State) -> Element<'_, Message> {
    let header = column![
        text("Suggestions")
            .size(16)
            .style(|_t| iced::widget::text::Style {
                color: Some(COL_TEXT),
            }),
        rule::horizontal(1).style(|_t| iced::widget::rule::Style {
            color: COL_MUTED,
            radius: 0.0.into(),
            fill_mode: iced::widget::rule::FillMode::Full,
            snap: true,
        }),
    ]
    .spacing(10);

    let body: Element<_> = if state.suggestions.is_empty() {
        container(
            text("No suggestions")
                .size(14)
                .style(|_t| iced::widget::text::Style {
                    color: Some(COL_MUTED),
                }),
        )
        .center_x(Fill)
        .center_y(Fill)
        .height(Fill)
        .into()
    } else {
        let items = state
            .suggestions
            .iter()
            .fold(Column::new().spacing(10), |col, s| {
                col.push(suggestion_card(s))
            });

        scrollable(container(items).padding(Padding::new(8.0))).height(Fill).into()
    };

    container(column![header, body].spacing(12))
        .width(Length::FillPortion(1))
        .height(Fill)
        .padding(Padding::new(16.0))
        .style(|_theme| container_bg(COL_PANEL, 0.0))
        .into()
}

fn suggestion_card<'a>(s: &'a crate::suggestion::Suggestion) -> Element<'a, Message> {
    let message = text(&s.message).size(12).style(|_t| iced::widget::text::Style {
        color: Some(COL_MUTED),
    });

    let original = text(&s.original).size(14).style(|_t| iced::widget::text::Style {
        color: Some(COL_DANGER),
    });

    let arrow = text(" → ").size(13).style(|_t| iced::widget::text::Style {
        color: Some(COL_MUTED),
    });

    let replacement = text(&s.replacement).size(14).style(|_t| iced::widget::text::Style {
        color: Some(COL_SUCCESS),
    });

    let accept = button(text("Accept").size(12))
        .on_press(Message::ApplySuggestion(s.id.clone()))
        .style(|theme: &Theme, status| {
            let palette = theme.extended_palette();
            match status {
                iced::widget::button::Status::Active => iced::widget::button::Style {
                    text_color: COL_BG,
                    ..iced::widget::button::Style::default()
                        .with_background(palette.success.strong.color)
                },
                _ => iced::widget::button::primary(theme, status),
            }
        });

    container(
        column![
            message,
            row![original, arrow, replacement].spacing(4),
            accept
        ]
        .spacing(10),
    )
    .padding(Padding::new(14.0))
    .style(|_theme| container_panel(COL_EDITOR_BG))
    .into()
}

fn settings_modal<'a>(base: Element<'a, Message>, state: &'a State) -> Element<'a, Message> {
    use iced::widget::stack;

    let content = settings_content(state);

    // Backdrop + centered modal
    let overlay = container(
        container(content)
            .padding(Padding::new(16.0))
            .style(|_theme| container_panel(COL_PANEL))
            .width(420)
            .height(360),
    )
    .width(Fill)
    .height(Fill)
    .center_x(Fill)
    .center_y(Fill)
    .style(|_theme| iced::widget::container::Style {
        text_color: None,
        background: Some(iced::Background::Color(iced::Color {
            a: 0.70,
            ..COL_BG
        })),
        border: iced::Border::default(),
        shadow: iced::Shadow::default(),
        snap: true,
    });

    stack![base, overlay].into()
}

fn settings_content(state: &State) -> Element<'_, Message> {
    let provider_row = row![
        provider_button("OpenAI", state.temp_provider == ApiProvider::OpenAI, Message::SelectProvider(ApiProvider::OpenAI)),
        provider_button(
            "OpenRouter",
            state.temp_provider == ApiProvider::OpenRouter,
            Message::SelectProvider(ApiProvider::OpenRouter),
        ),
    ]
    .spacing(10);

    let api_key_value = if state.temp_provider == ApiProvider::OpenAI {
        state.temp_openai_api_key.clone()
    } else {
        state.temp_openrouter_api_key.clone()
    };

    let api_key_input: Element<'_, Message> = if state.temp_provider == ApiProvider::OpenAI {
        text_input("sk-...", &api_key_value)
            .secure(!state.show_api_key)
            .on_input(Message::TempOpenAiKeyChanged)
            .into()
    } else {
        text_input("sk-or-...", &api_key_value)
            .secure(!state.show_api_key)
            .on_input(Message::TempOpenRouterKeyChanged)
            .into()
    };

    let model_input = text_input("Model", &state.temp_model).on_input(Message::TempModelChanged);

    let test_button = button(text(if state.is_testing { "Testing..." } else { "Test connection" }))
        .on_press(Message::StartTestConnection);

    let test_status: Element<'_, Message> = if state.test_status.is_empty() {
        iced::widget::Space::new().height(0.0).into()
    } else {
        text(&state.test_status)
            .size(12)
            .style(|_t| iced::widget::text::Style {
                color: Some(if state.test_status.contains("OK") {
                    COL_SUCCESS
                } else {
                    COL_DANGER
                }),
            })
            .into()
    };

    let buttons = row![
        button(text("Cancel")).on_press(Message::CloseSettings),
        iced::widget::Space::new().width(Fill),
        button(text("Save"))
            .on_press(Message::SaveSettings)
            .style(|theme, status| iced::widget::button::primary(theme, status)),
    ]
    .align_y(Alignment::Center)
    .spacing(10);

    container(
        column![
            text("Settings").size(20).style(|_t| iced::widget::text::Style { color: Some(COL_TEXT) }),
            text("API Provider").size(13).style(|_t| iced::widget::text::Style { color: Some(COL_TEXT) }),
            provider_row,
            text("API Key").size(13).style(|_t| iced::widget::text::Style { color: Some(COL_TEXT) }),
            row![
                button(text(if state.show_api_key { "🙈" } else { "👁" }))
                    .on_press(Message::ToggleShowApiKey)
                    .style(|theme, status| iced::widget::button::text(theme, status)),
                api_key_input,
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            text("Model").size(13).style(|_t| iced::widget::text::Style { color: Some(COL_TEXT) }),
            model_input,
            row![test_button, test_status].spacing(10).align_y(Alignment::Center),
            iced::widget::Space::new().height(10.0),
            buttons,
        ]
        .spacing(12)
        .padding(Padding::new(10.0)),
    )
    .width(Fill)
    .into()
}

fn provider_button(
    label: &'static str,
    selected: bool,
    message: Message,
) -> Element<'static, Message> {
    let btn = button(text(label).size(13))
        .on_press(message)
        .style(move |theme: &Theme, status| {
            let palette = theme.extended_palette();
            if selected {
                iced::widget::button::Style {
                    text_color: COL_BG,
                    ..iced::widget::button::Style::default().with_background(COL_ACCENT)
                }
            } else {
                match status {
                    iced::widget::button::Status::Hovered => iced::widget::button::Style {
                        text_color: COL_TEXT,
                        ..iced::widget::button::Style::default()
                            .with_background(palette.background.weak.color)
                    },
                    _ => iced::widget::button::Style {
                        text_color: COL_TEXT,
                        ..iced::widget::button::Style::default()
                            .with_background(COL_EDITOR_BG)
                    },
                }
            }
        });

    btn.into()
}

fn container_bg(bg: iced::Color, _radius: f32) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: None,
        background: Some(iced::Background::Color(bg)),
        border: iced::Border::default(),
        shadow: iced::Shadow::default(),
        snap: true,
    }
}

fn container_panel(bg: iced::Color) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: None,
        background: Some(iced::Background::Color(bg)),
        border: iced::Border {
            color: iced::Color {
                a: 0.15,
                ..COL_TEXT
            },
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: iced::Shadow {
            color: iced::Color {
                a: 0.35,
                ..iced::Color::BLACK
            },
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
        snap: true,
    }
}
