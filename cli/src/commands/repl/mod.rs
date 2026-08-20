//! The `repl` command - interactive REPL.

pub mod edit_mode;
pub mod highlighter;
pub mod lexer;

use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use bumpalo::Bump;
use edit_mode::{BufferState, MelbiEditMode};
use highlighter::Highlighter;
use melbi_core::parser::{ExpressionParser, Rule};
use melbi_core::types::manager::TypeManager;
use nu_ansi_term::Style;
use pest::Parser as PestParser;
use reedline::{
    DefaultCompleter, DefaultPrompt, DefaultPromptSegment, DescriptionMode, EditCommand, Emacs,
    FileBackedHistory, IdeMenu, KeyCode, KeyModifiers, Keybindings, MenuBuilder, Reedline,
    ReedlineEvent, ReedlineMenu, Signal, ValidationResult, default_emacs_keybindings,
};

use super::eval::interpret_input;
use crate::cli::ReplArgs;
use crate::common::engine::build_stdlib;
use crate::common::panic as panic_handler;

/// A `reedline` validator that uses the full Melbi parser to determine input completeness.
///
/// This validator provides accurate multi-line support by parsing the user's input
/// in real-time. If the parser encounters an "unexpected end of input" error, it
/// means the expression is incomplete, and the REPL will wait for more input.
///
/// Any other result, including a successful parse or a different syntax error,
/// considers the input `Complete` and ready for evaluation.
///
/// # Examples of Incomplete Input
///
/// - `1 +`
/// - `if true then "foo"`
/// - `[1, 2, 3,`
///
/// # Manual Newlines
///
/// To split a complete expression across multiple lines for readability,
/// users can press `Alt + Enter` to insert a newline manually.
pub struct MelbiValidator;

impl MelbiValidator {
    /// Returns `true` if the given input buffer is incomplete and expects more lines.
    #[must_use]
    pub fn is_incomplete(input: &str) -> bool {
        if input.trim().is_empty() {
            return false;
        }

        match ExpressionParser::parse(Rule::main, input) {
            Ok(_) => false,
            Err(e) => {
                let pest::error::InputLocation::Pos(pos) = e.location else {
                    return false;
                };
                if pos >= input.len() {
                    true
                } else if input[pos..].starts_with(['"', '\'']) {
                    // Assume its an unterminated string literal.
                    true
                } else {
                    // Syntax error within complete input
                    false
                }
            }
        }
    }
}

impl reedline::Validator for MelbiValidator {
    fn validate(&self, input: &str) -> ValidationResult {
        if Self::is_incomplete(input) {
            ValidationResult::Incomplete
        } else {
            ValidationResult::Complete
        }
    }
}

fn add_menu_keybindings(keybindings: &mut Keybindings) {
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );
    keybindings.add_binding(
        KeyModifiers::ALT,
        KeyCode::Enter,
        ReedlineEvent::Edit(vec![EditCommand::InsertNewline]),
    );
}

fn setup_reedline() -> (Reedline, DefaultPrompt) {
    // TODO: Populate with stdlib function names for tab completion
    let commands: Vec<String> = vec![];

    let completer = Box::new({
        let mut completions = DefaultCompleter::with_inclusions(&['-', '_']);
        completions.insert(commands);
        completions
    });

    // Use the interactive menu to select options from the completer
    let ide_menu = IdeMenu::default()
        .with_name("completion_menu")
        .with_min_completion_width(0)
        .with_max_completion_width(50)
        .with_max_completion_height(u16::MAX)
        .with_padding(0)
        .with_cursor_offset(0)
        .with_description_mode(DescriptionMode::PreferRight)
        .with_min_description_width(0)
        .with_max_description_width(50)
        .with_description_offset(1)
        .with_correct_cursor_pos(false);

    let completion_menu = Box::new(ide_menu);

    let mut keybindings = default_emacs_keybindings();
    add_menu_keybindings(&mut keybindings);

    let buffer_state = Arc::new(Mutex::new(BufferState::default()));

    let edit_mode = Box::new(MelbiEditMode::new(
        Emacs::new(keybindings),
        Arc::clone(&buffer_state),
    ));

    let history: Box<dyn reedline::History> = if let Some(h) = dirs::config_dir()
        .map(|p| p.join("melbi/history"))
        .and_then(|p| FileBackedHistory::with_file(10000, p).ok())
    {
        Box::new(h)
    } else {
        eprintln!("Warning: Could not initialize history file, using in-memory history");
        Box::new(FileBackedHistory::new(1000).unwrap())
    };

    let validator = Box::new(MelbiValidator);

    let line_editor = Reedline::create()
        .with_highlighter(Box::new(Highlighter::with_buffer_state(Arc::clone(
            &buffer_state,
        ))))
        .with_history(history)
        .with_validator(validator)
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_edit_mode(edit_mode);

    let prompt = DefaultPrompt::new(
        DefaultPromptSegment::Basic("  ".into()),
        DefaultPromptSegment::Empty,
    );

    (line_editor, prompt)
}

/// Run the REPL command.
#[must_use]
pub fn run(args: ReplArgs, no_color: bool) -> ExitCode {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let env = build_stdlib(&arena, type_manager);

    let (mut line_editor, prompt) = setup_reedline();

    let style = Style::new().dimmed();
    println!(
        "Melbi REPL. {}",
        style.paint("Ctrl+D to exit; Ctrl+C to abort entry")
    );

    loop {
        let sig = match line_editor.read_line(&prompt) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Reedline error: {e}");
                return ExitCode::SUCCESS;
            }
        };

        match sig {
            Signal::Success(buffer) => {
                if buffer.trim().is_empty() {
                    continue;
                }

                // Set current expression for panic handler (crash reports)
                panic_handler::set_current_expression(&buffer);

                // Run the expression - errors are printed by interpret_input
                // In REPL mode, we continue even on errors
                let _result = interpret_input(
                    type_manager,
                    &env,
                    buffer.as_ref(),
                    None, // REPL has no filename
                    args.runtime,
                    no_color,
                    args.time,
                );

                // Clear expression after evaluation (success or handled error)
                panic_handler::clear_current_expression();
            }
            Signal::CtrlD => {
                println!("\nGoodbye!");
                return ExitCode::SUCCESS;
            }
            _ => {
                continue;
            }
        }
    }
}
