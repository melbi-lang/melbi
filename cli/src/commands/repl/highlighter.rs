//! Syntax highlighter for the REPL using tree-sitter.

use std::sync::{Arc, Mutex};

use nu_ansi_term::{Color, Style};
use reedline::StyledText;
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent};

use super::edit_mode::BufferState;

#[derive(Debug)]
struct PaletteItem<'a> {
    name: &'a str,
    fg: Color,
    bg: Option<Color>,
}

const PALETTE: &[PaletteItem] = &[
    PaletteItem {
        name: "",
        fg: Color::White,
        bg: None,
    },
    PaletteItem {
        name: "keyword",
        fg: Color::Magenta,
        bg: None,
    },
    PaletteItem {
        name: "operator",
        fg: Color::White,
        bg: None,
    },
    PaletteItem {
        name: "constant",
        fg: Color::Cyan,
        bg: None,
    },
    PaletteItem {
        name: "number",
        fg: Color::Cyan,
        bg: None,
    },
    PaletteItem {
        name: "string",
        fg: Color::Green,
        bg: None,
    },
    PaletteItem {
        name: "comment",
        fg: Color::DarkGray,
        bg: None,
    },
    PaletteItem {
        name: "function",
        fg: Color::Blue,
        bg: None,
    },
    PaletteItem {
        name: "type",
        fg: Color::Yellow,
        bg: None,
    },
    PaletteItem {
        name: "variable",
        fg: Color::Red,
        bg: None,
    },
    PaletteItem {
        name: "property",
        fg: Color::Red,
        bg: None,
    },
    PaletteItem {
        name: "punctuation",
        fg: Color::White,
        bg: None,
    },
    PaletteItem {
        name: "embedded",
        fg: Color::White,
        bg: None,
    },
    PaletteItem {
        name: "error",
        fg: Color::White,
        bg: Some(Color::Rgb(0x80, 0x22, 0x3e)),
    },
];

const HIGHLIGHTS_QUERY: &str = include_str!("../../../../zed/languages/melbi/highlights.scm");

pub struct Highlighter {
    config: HighlightConfiguration,
    buffer_state: Option<Arc<Mutex<BufferState>>>,
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl Highlighter {
    /// Creates a new syntax highlighter.
    ///
    /// # Panics
    ///
    /// Panics if the tree-sitter highlight configuration fails to initialize.
    #[must_use]
    pub fn new() -> Self {
        Self::with_optional_buffer_state(None)
    }

    /// Creates a new syntax highlighter with shared buffer state tracking.
    ///
    /// # Panics
    ///
    /// Panics if the tree-sitter highlight configuration fails to initialize.
    #[must_use]
    pub fn with_buffer_state(buffer_state: Arc<Mutex<BufferState>>) -> Self {
        Self::with_optional_buffer_state(Some(buffer_state))
    }

    fn with_optional_buffer_state(buffer_state: Option<Arc<Mutex<BufferState>>>) -> Self {
        let highlight_names = PALETTE.iter().map(|item| item.name).collect::<Vec<_>>();

        let mut config = HighlightConfiguration::new(
            tree_sitter_melbi::LANGUAGE.into(),
            "melbi",
            HIGHLIGHTS_QUERY,
            "",
            "",
        )
        .expect("Failed to create highlight configuration");
        config.configure(&highlight_names);
        Self {
            config,
            buffer_state,
        }
    }
}

impl reedline::Highlighter for Highlighter {
    fn highlight(&self, line: &str, cursor: usize) -> StyledText {
        if let Some(state) = &self.buffer_state
            && let Ok(mut lock) = state.lock()
        {
            lock.buffer = line.to_string();
            lock.cursor = cursor;
        }

        let mut output = StyledText::new();

        let mut highlighter = tree_sitter_highlight::Highlighter::new();
        let Ok(highlights) = highlighter.highlight(&self.config, line.as_bytes(), None, |_| None)
        else {
            let style = Style::new().fg(PALETTE[0].fg);
            output.push((style, line.to_string()));
            return output;
        };

        let mut curr_style = Style::new().fg(PALETTE[0].fg);
        let mut curr_end = 0;

        for event in highlights {
            match event {
                Ok(HighlightEvent::HighlightStart(highlight)) => {
                    if let Some(item) = PALETTE.get(highlight.0) {
                        let mut style = Style::new().fg(item.fg);
                        if let Some(bg) = item.bg {
                            style = style.on(bg);
                        }
                        curr_style = style;
                    }
                }
                Ok(HighlightEvent::Source { start, end }) => {
                    let text = line[start..end].to_string();
                    output.push((curr_style, text));
                    curr_end = end;
                }
                Ok(HighlightEvent::HighlightEnd) => {
                    curr_style = Style::new().fg(PALETTE[0].fg);
                }
                Err(_) => {
                    let style = Style::new().fg(PALETTE[0].fg);
                    let text = line.get(curr_end..).unwrap_or_default().to_string();
                    output.push((style, text));
                    break;
                }
            }
        }

        output
    }
}
