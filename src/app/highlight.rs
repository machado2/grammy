use std::ops::Range;

use iced::advanced::text::highlighter::Format;
use iced::advanced::text::Highlighter;
use iced::{Color, Font, Theme};

use crate::suggestion::{Severity, Suggestion};

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub kind: Highlight,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub line_starts: Vec<usize>,
    pub spans: Vec<Span>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Highlight {
    Error,      // Red - grammar errors, typos
    Warning,    // Orange - awkward phrasing
    Suggestion, // Yellow - minor improvements
    Hovered,    // Blue - currently hovered
}

pub fn compute_line_starts(text: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    starts.push(0);

    for (i, ch) in text.char_indices() {
        if ch == '\n' {
            starts.push(i + 1);
        }
    }

    starts
}

pub fn spans_from_suggestions(suggestions: &[Suggestion], hovered_id: Option<&str>) -> Vec<Span> {
    suggestions
        .iter()
        .filter_map(|s| {
            if s.length == 0 {
                return None;
            }

            let kind = if hovered_id == Some(s.id.as_str()) {
                Highlight::Hovered
            } else {
                match s.severity {
                    Severity::Error => Highlight::Error,
                    Severity::Warning => Highlight::Warning,
                    Severity::Suggestion => Highlight::Suggestion,
                }
            };

            Some(Span {
                start: s.offset,
                end: s.offset + s.length,
                kind,
            })
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct SuggestionHighlighter {
    settings: Settings,
    current_line: usize,
}

impl Highlighter for SuggestionHighlighter {
    type Settings = Settings;
    type Highlight = Highlight;

    type Iterator<'a>
        = std::vec::IntoIter<(Range<usize>, Self::Highlight)>
    where
        Self: 'a;

    fn new(settings: &Self::Settings) -> Self {
        Self {
            settings: settings.clone(),
            current_line: 0,
        }
    }

    fn update(&mut self, new_settings: &Self::Settings) {
        // Settings changed -> ensure the editor re-feeds lines from the start
        if *new_settings != self.settings {
            self.current_line = 0;
        }
        self.settings = new_settings.clone();
    }

    fn change_line(&mut self, line: usize) {
        self.current_line = line;
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let line_index = self.current_line;
        self.current_line = self.current_line.saturating_add(1);

        let start_offset = self
            .settings
            .line_starts
            .get(line_index)
            .copied()
            .unwrap_or(0);

        let line_len = line.len();

        if line_len == 0 {
            return Vec::new().into_iter();
        }

        let line_end = start_offset + line_len;

        // Find spans that overlap this line
        let mut relevant_spans: Vec<(usize, usize, Highlight)> = Vec::new();
        for span in &self.settings.spans {
            if span.end <= start_offset || span.start >= line_end {
                continue;
            }
            let local_start = span.start.saturating_sub(start_offset).min(line_len);
            let local_end = span.end.saturating_sub(start_offset).min(line_len);
            if local_start < local_end {
                relevant_spans.push((local_start, local_end, span.kind));
            }
        }

        // If no spans overlap, no highlighting for this line
        if relevant_spans.is_empty() {
            return Vec::new().into_iter();
        }

        // Build only highlighted segments (do not emit Normal segments)
        let mut segments: Vec<(Range<usize>, Highlight)> = Vec::new();

        // Sort spans by start position
        relevant_spans.sort_by_key(|(start, _, _)| *start);

        for (span_start, span_end, kind) in relevant_spans {
            if span_start < span_end {
                segments.push((span_start..span_end, kind));
            }
        }

        segments.into_iter()
    }

    fn current_line(&self) -> usize {
        self.current_line
    }
}

pub fn to_format(highlight: &Highlight, _theme: &Theme) -> Format<Font> {
    // Severity-based colors for text highlighting
    let error: Color = Color {
        r: 1.0,
        g: 0.35,
        b: 0.35,
        a: 1.0,
    }; // Red
    let warning: Color = Color {
        r: 1.0,
        g: 0.6,
        b: 0.2,
        a: 1.0,
    }; // Orange
    let suggestion: Color = Color {
        r: 1.0,
        g: 0.85,
        b: 0.3,
        a: 1.0,
    }; // Yellow
    let hovered: Color = Color {
        r: 0.25,
        g: 0.75,
        b: 1.0,
        a: 1.0,
    }; // Blue

    match highlight {
        Highlight::Error => Format {
            color: Some(error),
            font: None,
        },
        Highlight::Warning => Format {
            color: Some(warning),
            font: None,
        },
        Highlight::Suggestion => Format {
            color: Some(suggestion),
            font: None,
        },
        Highlight::Hovered => Format {
            color: Some(hovered),
            font: None,
        },
    }
}
