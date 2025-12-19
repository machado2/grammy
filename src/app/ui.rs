use iced::widget::text::Wrapping;
use iced::widget::{
    button, column, container, mouse_area, row, rule, scrollable, slider, text, text_editor,
    text_input, Column,
};
use iced::{Alignment, Background, Border, Color, Element, Fill, Length, Padding, Theme};

use crate::config::ApiProvider;

use super::state::{Message, State};
use super::style::{
    btn_ghost, btn_primary, btn_secondary, btn_success, editor_style, glass_container,
    glass_editor, rule_muted, text_input as style_text_input, COL_BG, COL_DANGER, COL_MUTED,
    COL_SUCCESS, COL_TEXT,
};
use super::{highlight, highlight::SuggestionHighlighter};

pub(super) fn view(state: &State) -> Element<'_, Message> {
    let header = row![
        text("Grammy")
            .size(24)
            .font(iced::Font {
                weight: iced::font::Weight::Bold,
                ..Default::default()
            })
            .style(|_t| iced::widget::text::Style {
                color: Some(COL_TEXT),
            }),
        iced::widget::Space::new().width(Fill),
        button(text("⚙ Settings").size(14))
            .on_press(Message::OpenSettings)
            .padding(Padding::new(8.0))
            .style(btn_ghost),
    ]
    .align_y(Alignment::Center)
    .padding(Padding::new(20.0));

    let status_color = if state.status.contains("error") || state.status.contains("Error") {
        COL_DANGER
    } else if state.status == "All good!" {
        COL_SUCCESS
    } else {
        COL_MUTED
    };

    let status_bar = row![
        text(&state.status)
            .size(12)
            .style(move |_t| iced::widget::text::Style {
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
    .padding(Padding::new(12.0));

    let suggestions_panel = suggestions_sidebar(state);
    let editor_panel = editor(state);

    let main = row![editor_panel, suggestions_panel,]
        .spacing(20)
        .height(Fill)
        .width(Fill)
        .padding(Padding::from([0.0, 20.0]));

    let root = column![header, main, status_bar]
        .width(Fill)
        .height(Fill)
        .spacing(0)
        .align_x(Alignment::Start);

    let base =
        container(root)
            .width(Fill)
            .height(Fill)
            .style(|_theme| iced::widget::container::Style {
                background: Some(Background::Color(COL_BG)),
                text_color: Some(COL_TEXT),
                ..Default::default()
            });

    if state.show_settings {
        settings_modal(base.into(), state)
    } else {
        base.into()
    }
}

fn editor(state: &State) -> Element<'_, Message> {
    let title = text("Your text")
        .size(14)
        .style(|_t| iced::widget::text::Style {
            color: Some(COL_MUTED),
        });

    let full_text = state.editor.text();
    let line_starts = highlight::compute_line_starts(&full_text);
    let spans =
        highlight::spans_from_suggestions(&state.suggestions, state.hovered_suggestion.as_deref());
    let settings = highlight::Settings { line_starts, spans };

    let editor = text_editor(&state.editor)
        .placeholder("Paste or type here...")
        .on_action(Message::EditorAction)
        .highlight_with::<SuggestionHighlighter>(settings, highlight::to_format)
        .height(Fill)
        .padding(16)
        .size(16)
        .style(editor_style);

    let frame = container(editor)
        .width(Fill)
        .height(Fill)
        .padding(Padding::new(4.0))
        .style(glass_editor);

    column![title, frame]
        .spacing(12)
        .width(Length::FillPortion(3))
        .height(Fill)
        .into()
}

fn suggestions_sidebar(state: &State) -> Element<'_, Message> {
    let header = column![
        row![
            text("Suggestions")
                .size(18)
                .style(|_t| iced::widget::text::Style {
                    color: Some(COL_TEXT),
                }),
            iced::widget::Space::new().width(Fill),
            button(text("Check again").size(12))
                .on_press(Message::ForceCheck)
                .padding(Padding::from([6.0, 12.0]))
                .style(btn_secondary),
        ]
        .align_y(Alignment::Center)
        .spacing(10),
        rule::horizontal(1).style(rule_muted),
    ]
    .spacing(16);

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
            text("No suggestions found.\nGreat job!")
                .align_x(Alignment::Center)
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
            .fold(Column::new().spacing(16), |col, s| {
                let hovered = state.hovered_suggestion.as_deref() == Some(s.id.as_str());

                let card = suggestion_card(s, hovered);
                let card = mouse_area(card)
                    .on_enter(Message::HoverSuggestion(s.id.clone()))
                    .on_exit(Message::ClearHoverSuggestion);

                col.push(card)
            });

        scrollable(container(items).padding(Padding::new(4.0)))
            .height(Fill)
            .into()
    };

    container(column![header, body].spacing(16))
        .width(Length::FillPortion(2))
        .height(Fill)
        .padding(Padding::new(20.0))
        .style(glass_container)
        .into()
}

fn suggestion_card<'a>(
    s: &'a crate::suggestion::Suggestion,
    hovered: bool,
) -> Element<'a, Message> {
    let message = text(&s.message)
        .size(13)
        .style(|_t| iced::widget::text::Style {
            color: Some(COL_MUTED),
        });

    let original = text(&s.original)
        .size(14)
        .wrapping(Wrapping::WordOrGlyph)
        .width(Fill)
        .style(|_t| iced::widget::text::Style {
            color: Some(COL_DANGER),
        });

    let (diff_row, actions) = if let Some(ref replacement_text) = s.replacement {
        let arrow = text("→").size(14).style(|_t| iced::widget::text::Style {
            color: Some(COL_MUTED),
        });

        let replacement = text(replacement_text)
            .size(14)
            .wrapping(Wrapping::WordOrGlyph)
            .width(Fill)
            .style(|_t| iced::widget::text::Style {
                color: Some(COL_SUCCESS),
            });

        let accept = button(text("Accept").size(12))
            .on_press(Message::ApplySuggestion(s.id.clone()))
            .padding(Padding::from([8.0, 16.0]))
            .style(btn_success)
            .width(Fill);

        let dismiss = button(text("Dismiss").size(12))
            .on_press(Message::DismissSuggestion(s.id.clone()))
            .padding(Padding::from([8.0, 16.0]))
            .style(btn_ghost)
            .width(Fill);

        let row_content = row![
            container(original).width(Length::FillPortion(1)),
            container(arrow).center_x(Fill),
            container(replacement).width(Length::FillPortion(1)),
        ]
        .spacing(8)
        .width(Fill)
        .align_y(Alignment::Center);

        let action_row = row![dismiss, accept].spacing(12);
        (row_content, action_row)
    } else {
        // Comment only
        let dismiss = button(text("Dismiss").size(12))
            .on_press(Message::DismissSuggestion(s.id.clone()))
            .padding(Padding::from([8.0, 16.0]))
            .style(btn_ghost)
            .width(Fill);

        let row_content = row![container(original).width(Fill)]
            .width(Fill)
            .align_y(Alignment::Center);

        (row_content, row![dismiss])
    };

    container(
        column![
            message,
            diff_row,
            iced::widget::Space::new().height(4.0),
            actions
        ]
        .spacing(12),
    )
    .padding(Padding::new(16.0))
    .style(move |_theme| {
        let alpha = if hovered { 0.1 } else { 0.0 };
        iced::widget::container::Style {
            background: Some(Background::Color(Color {
                a: alpha,
                ..Color::WHITE
            })),
            border: Border {
                color: Color {
                    a: 0.1,
                    ..Color::WHITE
                },
                width: 1.0,
                radius: 12.0.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

fn settings_modal<'a>(base: Element<'a, Message>, state: &'a State) -> Element<'a, Message> {
    use iced::widget::stack;

    let content = settings_content(state);

    let overlay = container(
        container(content)
            .padding(Padding::new(24.0))
            .style(glass_container)
            .width(450)
            .height(Length::Shrink),
    )
    .width(Fill)
    .height(Fill)
    .center_x(Fill)
    .center_y(Fill)
    .style(|_theme| iced::widget::container::Style {
        background: Some(Background::Color(Color { a: 0.8, ..COL_BG })),
        ..Default::default()
    });

    stack![base, overlay].into()
}

fn settings_content(state: &State) -> Element<'_, Message> {
    let provider_row = row![
        provider_button(
            "OpenAI",
            state.temp_provider == ApiProvider::OpenAI,
            Message::SelectProvider(ApiProvider::OpenAI)
        ),
        provider_button(
            "OpenRouter",
            state.temp_provider == ApiProvider::OpenRouter,
            Message::SelectProvider(ApiProvider::OpenRouter),
        ),
    ]
    .spacing(12);

    let api_key_value = if state.temp_provider == ApiProvider::OpenAI {
        state.temp_openai_api_key.clone()
    } else {
        state.temp_openrouter_api_key.clone()
    };

    let api_key_input: Element<'_, Message> = if state.temp_provider == ApiProvider::OpenAI {
        text_input("sk-...", &api_key_value)
            .secure(!state.show_api_key)
            .on_input(Message::TempOpenAiKeyChanged)
            .style(style_text_input)
            .into()
    } else {
        text_input("sk-or-...", &api_key_value)
            .secure(!state.show_api_key)
            .on_input(Message::TempOpenRouterKeyChanged)
            .style(style_text_input)
            .into()
    };

    let model_input = text_input("Model", &state.temp_model)
        .on_input(Message::TempModelChanged)
        .style(style_text_input);

    let test_button = button(text(if state.is_testing {
        "Testing..."
    } else {
        "Test connection"
    }))
    .on_press(Message::StartTestConnection)
    .padding(Padding::from([8.0, 16.0]))
    .style(btn_secondary)
    .width(Fill);

    let debounce_val = state.temp_debounce_ms;
    let debounce_text = if debounce_val > 5000.0 {
        "Never".to_string()
    } else {
        format!("{:.1}s", debounce_val / 1000.0)
    };

    let debounce_slider = row![
        slider(250.0..=5500.0, debounce_val, Message::TempDebounceChanged)
            .step(250.0)
            .width(Fill),
        text(debounce_text).size(14).width(Length::Fixed(50.0)),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

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
            .style(btn_ghost),
        iced::widget::Space::new().width(Fill),
        button(text("Save Settings"))
            .on_press(Message::SaveSettings)
            .padding(Padding::from([8.0, 16.0]))
            .style(btn_primary),
    ]
    .align_y(Alignment::Center)
    .spacing(12);

    column![
        text("Settings")
            .size(22)
            .style(|_t| iced::widget::text::Style {
                color: Some(COL_TEXT)
            }),
        iced::widget::Space::new().height(4.0),
        text("API Provider")
            .size(14)
            .style(|_t| iced::widget::text::Style {
                color: Some(COL_TEXT)
            }),
        provider_row,
        text("API Key")
            .size(14)
            .style(|_t| iced::widget::text::Style {
                color: Some(COL_TEXT)
            }),
        row![
            api_key_input,
            button(text(if state.show_api_key { "🙈" } else { "👁" }))
                .on_press(Message::ToggleShowApiKey)
                .padding(Padding::new(10.0))
                .style(btn_ghost),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        text("Model")
            .size(14)
            .style(|_t| iced::widget::text::Style {
                color: Some(COL_TEXT)
            }),
        model_input,
        iced::widget::Space::new().height(4.0),
        text("Auto-check Delay")
            .size(14)
            .style(|_t| iced::widget::text::Style {
                color: Some(COL_TEXT)
            }),
        debounce_slider,
        iced::widget::Space::new().height(4.0),
        test_button,
        test_status,
        iced::widget::Space::new().height(16.0),
        buttons,
    ]
    .spacing(16)
    .into()
}

fn provider_button(
    label: &'static str,
    selected: bool,
    message: Message,
) -> Element<'static, Message> {
    let btn = button(text(label).size(13).align_x(Alignment::Center))
        .on_press(message)
        .padding(Padding::from([10.0, 16.0]))
        .width(Fill)
        .style(move |theme: &Theme, status| {
            if selected {
                btn_primary(theme, status)
            } else {
                btn_secondary(theme, status)
            }
        });

    btn.into()
}
