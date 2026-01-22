//! The `repl` command - interactive REPL.

pub mod highlighter;
pub mod lexer;

use bumpalo::Bump;
use melbi_core::{
    parser::{ExpressionParser, Rule},
    types::manager::TypeManager,
};
use nu_ansi_term::Style;
use pest::Parser as PestParser;
use reedline::{
    DefaultCompleter, DefaultPrompt, DefaultPromptSegment, DescriptionMode, EditCommand, Emacs,
    FileBackedHistory, IdeMenu, KeyCode, KeyModifiers, Keybindings, MenuBuilder, Reedline,
    ReedlineEvent, ReedlineMenu, Signal, ValidationResult, default_emacs_keybindings,
};

use crate::cli::ReplArgs;
use crate::common::{CliResult, engine::build_stdlib, panic as panic_handler};
use highlighter::Highlighter;
use lexer::calculate_depth;

use super::eval::interpret_input;

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
struct MelbiValidator;

impl reedline::Validator for MelbiValidator {
    fn validate(&self, input: &str) -> ValidationResult {
        match ExpressionParser::parse(Rule::main, input) {
            Ok(_) => ValidationResult::Complete,
            Err(e) => {
                let pest::error::InputLocation::Pos(pos) = e.location else {
                    return ValidationResult::Complete;
                };
                if pos >= input.len() {
                    ValidationResult::Incomplete
                } else if input[pos..].starts_with(&['"', '\'']) {
                    // Assume its an unterminated string literal.
                    ValidationResult::Incomplete
                } else {
                    // Assume it's complete, but contains a syntax error.
                    ValidationResult::Complete
                }
            }
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
    let commands: Vec<String> = vec![];

    let completer = Box::new({
        let mut completions = DefaultCompleter::with_inclusions(&['-', '_']);
        completions.insert(commands.clone());
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
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Enter,
        ReedlineEvent::Multiple(vec![
            ReedlineEvent::Enter,
            ReedlineEvent::ExecuteHostCommand("!indent".into()),
        ]),
    );
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Char('}'),
        ReedlineEvent::ExecuteHostCommand("!dedent".into()),
    );

    let edit_mode = Box::new(Emacs::new(keybindings));

    let history: Box<dyn reedline::History> = match dirs::config_dir()
        .map(|p| p.join("melbi/history"))
        .and_then(|p| FileBackedHistory::with_file(10000, p).ok())
    {
        Some(h) => Box::new(h),
        None => {
            eprintln!("Warning: Could not initialize history file, using in-memory history");
            Box::new(FileBackedHistory::new(1000).unwrap())
        }
    };

    let validator = Box::new(MelbiValidator);

    let line_editor = Reedline::create()
        .with_highlighter(Box::new(Highlighter::new()))
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
pub fn run(args: ReplArgs, no_color: bool) -> CliResult<()> {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let (globals_types, globals_values) = build_stdlib(&arena, type_manager);

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
                return Ok(());
            }
        };

        match sig {
            Signal::Success(cmd) if cmd == "!indent" => {
                let buffer = line_editor.current_buffer_contents();
                if let Some(depth) = calculate_depth(buffer) {
                    if depth > 0 {
                        line_editor
                            .run_edit_commands(&[EditCommand::InsertString("    ".repeat(depth))]);
                    }
                }
                continue;
            }
            Signal::Success(cmd) if cmd == "!dedent" => {
                let buffer = line_editor.current_buffer_contents();

                // Check if line is effectively empty (only whitespace)
                let is_blank_line = buffer.lines().last().map_or(false, |l| l.trim().is_empty());

                if is_blank_line {
                    let Some(current_depth) = calculate_depth(&buffer) else {
                        continue;
                    };
                    let target_depth = current_depth.saturating_sub(1); // Dedent level

                    line_editor.run_edit_commands(&[
                        EditCommand::MoveToLineStart { select: false },
                        EditCommand::ClearToLineEnd,
                        EditCommand::InsertString("    ".repeat(target_depth)),
                        EditCommand::InsertChar('}'),
                    ]);
                } else {
                    // Cursor is after code (e.g. `let x = {`), just insert `}`
                    line_editor.run_edit_commands(&[EditCommand::InsertChar('}')]);
                }
                continue;
            }
            Signal::Success(buffer) => {
                // Set current expression for panic handler (crash reports)
                panic_handler::set_current_expression(&buffer);

                let result = interpret_input(
                    type_manager,
                    globals_types,
                    globals_values,
                    buffer.as_ref(),
                    None, // REPL has no filename
                    args.runtime,
                    no_color,
                );

                // Clear expression after evaluation (success or handled error)
                panic_handler::clear_current_expression();

                result?;
            }
            Signal::CtrlD => {
                println!("\nGoodbye!");
                return Ok(());
            }
            Signal::CtrlC => {
                continue;
            }
        }
    }
}
