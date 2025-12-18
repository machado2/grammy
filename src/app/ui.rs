use iced::widget::{
    button, column, container, mouse_area, row, rule, scrollable, text, text_editor, text_input,
    Column,
};
use iced::widget::text::Wrapping;
use iced::{Alignment, Element, Fill, Length, Padding, Theme};

use crate::config::ApiProvider;

use super::state::{Message, State};
use super::style::{
    btn_ghost, btn_primary, btn_secondary, btn_success,
    COL_ACCENT, COL_BG, COL_DANGER, COL_EDITOR_BG, COL_MUTED, COL_PANEL, COL_SUCCESS, COL_TEXT,
};
use super::{highlight, highlight::SuggestionHighlighter};

pub(super) fn view(state: &State) -> Element<'_, Message> {
    let header = container(
        row![
            text("Grammy").size(22).style(|_t| iced::widget::text::Style {
                color: Some(COL_TEXT),
            }),
            iced::widget::Space::new().width(Fill),
            button(text("⚙").size(18))
                .on_press(Message::OpenSettings)
                .padding(Padding::new(8.0))
                .style(btn_ghost),
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

fn container_panel_hovered(bg: iced::Color) -> iced::widget::container::Style {
    let mut style = container_panel(bg);
    style.border.color = COL_ACCENT;
    style.border.width = 1.0;
    style
}

fn editor(state: &State) -> Element<'_, Message> {
    let title = text("Your text")
        .size(12)
        .style(|_t| iced::widget::text::Style {
            color: Some(COL_MUTED),
        });

    let full_text = state.editor.text();
    let line_starts = highlight::compute_line_starts(&full_text);
    let spans = highlight::spans_from_suggestions(
        &state.suggestions,
        state.hovered_suggestion.as_deref(),
    );
    let settings = highlight::Settings { line_starts, spans };

    let editor = text_editor(&state.editor)
        .placeholder("Paste or type here...")
        .on_action(Message::EditorAction)
        .highlight_with::<SuggestionHighlighter>(settings, highlight::to_format)
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
        row![
            text("Suggestions")
                .size(16)
                .style(|_t| iced::widget::text::Style {
                    color: Some(COL_TEXT),
                }),
            iced::widget::Space::new().width(Fill),
            button(text("Check again").size(12))
                .on_press(Message::ForceCheck)
                .padding(Padding::from([6.0, 10.0]))
                .style(btn_secondary),
        ]
        .align_y(Alignment::Center)
        .spacing(10),
        rule::horizontal(1).style(|_t| iced::widget::rule::Style {
            color: COL_MUTED,
            radius: 0.0.into(),
            fill_mode: iced::widget::rule::FillMode::Full,
            snap: true,
        }),
    ]
    .spacing(10);

    let body: Element<_> = if state.is_checking {
        container(
            text("Checking...")
                .size(14)
                .style(|_t| iced::widget::text::Style {
                    color: Some(COL_MUTED),
                }),
        )
        .center_x(Fill)
        .center_y(Fill)
        .height(Fill)
        .into()
    } else if state.suggestions.is_empty() {
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
                let hovered = state.hovered_suggestion.as_deref() == Some(s.id.as_str());

                let card = suggestion_card(s, hovered);
                let card = mouse_area(card)
                    .on_enter(Message::HoverSuggestion(s.id.clone()))
                    .on_exit(Message::ClearHoverSuggestion);

                col.push(card)
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

fn suggestion_card<'a>(s: &'a crate::suggestion::Suggestion, hovered: bool) -> Element<'a, Message> {
    let message = text(&s.message).size(12).style(|_t| iced::widget::text::Style {
        color: Some(COL_MUTED),
    });

    let original = text(&s.original)
        .size(14)
        .wrapping(Wrapping::WordOrGlyph)
        .width(Fill)
        .style(|_t| iced::widget::text::Style {
            color: Some(COL_DANGER),
        });

    let arrow = text("→").size(13).style(|_t| iced::widget::text::Style {
        color: Some(COL_MUTED),
    });

    let replacement = text(&s.replacement)
        .size(14)
        .wrapping(Wrapping::WordOrGlyph)
        .width(Fill)
        .style(|_t| iced::widget::text::Style {
            color: Some(COL_SUCCESS),
        });

    let accept = button(text("Accept").size(12))
        .on_press(Message::ApplySuggestion(s.id.clone()))
        .padding(Padding::from([6.0, 12.0]))
        .style(btn_success);

    let dismiss = button(text("Dismiss").size(12))
        .on_press(Message::DismissSuggestion(s.id.clone()))
        .padding(Padding::from([6.0, 12.0]))
        .style(btn_ghost);

    let actions = row![accept, dismiss].spacing(8);

    let diff_row = row![
        container(original).width(Length::FillPortion(5)),
        container(arrow)
            .width(Length::FillPortion(1))
            .center_x(Fill),
        container(replacement).width(Length::FillPortion(5)),
    ]
    .spacing(8)
    .width(Fill);

    container(
        column![
            message,
            diff_row,
            actions
        ]
        .spacing(10),
    )
    .padding(Padding::new(14.0))
    .style(move |_theme| {
        if hovered {
            container_panel_hovered(COL_EDITOR_BG)
        } else {
            container_panel(COL_EDITOR_BG)
        }
    })
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
            .height(Length::Shrink),
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
        .on_press(Message::StartTestConnection)
        .padding(Padding::from([8.0, 16.0]))
        .style(btn_secondary);

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
        button(text("Cancel"))
            .on_press(Message::CloseSettings)
            .padding(Padding::from([8.0, 16.0]))
            .style(btn_secondary),
        iced::widget::Space::new().width(Fill),
        button(text("Save"))
            .on_press(Message::SaveSettings)
            .padding(Padding::from([8.0, 16.0]))
            .style(btn_primary),
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
                    .padding(Padding::new(8.0))
                    .style(btn_ghost),
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
        .padding(Padding::from([8.0, 16.0]))
        .style(move |theme: &Theme, status| {
            if selected {
                btn_primary(theme, status)
            } else {
                btn_secondary(theme, status)
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
