//! Custom edit mode for the Melbi REPL with smart indentation and dedentation.

use std::sync::{Arc, Mutex};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use reedline::{EditCommand, EditMode, Emacs, PromptEditMode, ReedlineEvent, ReedlineRawEvent};

use super::MelbiValidator;
use super::lexer::calculate_depth;

/// Shared buffer state tracked by the highlighter and edit mode.
#[derive(Debug, Default, Clone)]
pub struct BufferState {
    /// The full current text content in the editor buffer.
    pub buffer: String,
    /// The current cursor byte position in the buffer.
    pub cursor: usize,
}

impl BufferState {
    /// Inserts a character at the current cursor position and advances the cursor.
    pub fn insert_char(&mut self, c: char) {
        if self.cursor >= self.buffer.len() {
            self.buffer.push(c);
            self.cursor = self.buffer.len();
        } else {
            self.buffer.insert(self.cursor, c);
            self.cursor += c.len_utf8();
        }
    }

    /// Inserts a string at the current cursor position and advances the cursor.
    pub fn insert_str(&mut self, s: &str) {
        if self.cursor >= self.buffer.len() {
            self.buffer.push_str(s);
            self.cursor = self.buffer.len();
        } else {
            self.buffer.insert_str(self.cursor, s);
            self.cursor += s.len();
        }
    }

    /// Deletes the character before the cursor (backspace behavior).
    pub fn backspace(&mut self) {
        if self.cursor > 0 && !self.buffer.is_empty() {
            let prev_char_idx = self.buffer[..self.cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.buffer.drain(prev_char_idx..self.cursor);
            self.cursor = prev_char_idx;
        }
    }

    /// Resets the buffer state.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = 0;
    }
}

/// Custom edit mode wrapping standard Emacs editing with smart indentation.
pub struct MelbiEditMode {
    emacs: Emacs,
    buffer_state: Arc<Mutex<BufferState>>,
}

impl MelbiEditMode {
    /// Creates a new `MelbiEditMode`.
    #[must_use]
    pub fn new(emacs: Emacs, buffer_state: Arc<Mutex<BufferState>>) -> Self {
        Self {
            emacs,
            buffer_state,
        }
    }
}

impl EditMode for MelbiEditMode {
    fn parse_event(&mut self, event: ReedlineRawEvent) -> ReedlineEvent {
        let crossterm_event: Event = event.into();
        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = crossterm_event
        {
            let mut state = self.buffer_state.lock().unwrap();

            // Enter on incomplete expression: auto-indent next line
            if code == KeyCode::Enter && modifiers == KeyModifiers::NONE {
                if !MelbiValidator::is_incomplete(&state.buffer) {
                    state.clear();
                    return ReedlineEvent::Enter;
                }
                let depth = calculate_depth(&state.buffer).unwrap_or(0);
                let indent = "    ".repeat(depth);
                state.insert_char('\n');
                state.insert_str(&indent);

                if depth > 0 {
                    return ReedlineEvent::Edit(vec![
                        EditCommand::InsertNewline,
                        EditCommand::InsertString(indent),
                    ]);
                }
                return ReedlineEvent::Edit(vec![EditCommand::InsertNewline]);
            }

            // Alt+Enter: manual newline with auto-indentation
            if code == KeyCode::Enter && modifiers == KeyModifiers::ALT {
                let depth = calculate_depth(&state.buffer).unwrap_or(0);
                let indent = "    ".repeat(depth);
                state.insert_char('\n');
                state.insert_str(&indent);

                if depth > 0 {
                    return ReedlineEvent::Edit(vec![
                        EditCommand::InsertNewline,
                        EditCommand::InsertString(indent),
                    ]);
                }
                return ReedlineEvent::Edit(vec![EditCommand::InsertNewline]);
            }

            // Closing delimiters: '}', ']', ')' on an indented blank line dedent by 1 level
            if let KeyCode::Char(c @ ('}' | ']' | ')')) = code
                && (modifiers == KeyModifiers::NONE || modifiers == KeyModifiers::SHIFT)
            {
                let text_before_cursor = &state.buffer[..state.cursor.min(state.buffer.len())];
                let current_line = text_before_cursor.lines().last().unwrap_or("");
                if current_line.trim().is_empty() && !current_line.is_empty() {
                    let current_depth = calculate_depth(text_before_cursor).unwrap_or(0);
                    let target_depth = current_depth.saturating_sub(1);
                    let indent = "    ".repeat(target_depth);

                    let line_start = text_before_cursor.rfind('\n').map(|i| i + 1).unwrap_or(0);
                    let cursor = state.cursor;
                    state.buffer.drain(line_start..cursor);
                    state.cursor = line_start;
                    state.insert_str(&indent);
                    state.insert_char(c);

                    return ReedlineEvent::Edit(vec![
                        EditCommand::MoveToLineStart { select: false },
                        EditCommand::ClearToLineEnd,
                        EditCommand::InsertString(indent),
                        EditCommand::InsertChar(c),
                    ]);
                }
                state.insert_char(c);
            } else if let KeyCode::Char(c) = code
                && (modifiers == KeyModifiers::NONE || modifiers == KeyModifiers::SHIFT)
            {
                state.insert_char(c);
            } else if code == KeyCode::Backspace && modifiers == KeyModifiers::NONE {
                state.backspace();
            } else if (code == KeyCode::Char('c') || code == KeyCode::Char('u'))
                && modifiers == KeyModifiers::CONTROL
            {
                state.clear();
            }
        }

        if let Ok(raw) = ReedlineRawEvent::try_from(crossterm_event) {
            self.emacs.parse_event(raw)
        } else {
            ReedlineEvent::None
        }
    }

    fn edit_mode(&self) -> PromptEditMode {
        self.emacs.edit_mode()
    }
}
