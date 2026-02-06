use bumpalo::Bump;
use clap::{Parser, ValueEnum};
use melbi::{RenderConfig, render_error_to};
use melbi_cli::{highlighter::Highlighter, lexer::calculate_depth};
use melbi_core::{
    analyzer::analyze,
    api::EnvironmentBuilder,
    compiler::BytecodeCompiler,
    evaluator::{Evaluator, EvaluatorOptions},
    parser::{self, ExpressionParser, Rule},
    stdlib::register_stdlib,
    types::{Type, manager::TypeManager},
    values::{binder::Binder, dynamic::Value},
    vm::VM,
};
use miette::Result;
use nu_ansi_term::Style;
use pest::Parser as PestParser;
use reedline::{
    DefaultCompleter, DefaultPrompt, DefaultPromptSegment, DescriptionMode, EditCommand, Emacs,
    FileBackedHistory, IdeMenu, KeyCode, KeyModifiers, Keybindings, MenuBuilder, Reedline,
    ReedlineEvent, ReedlineMenu, Signal, ValidationResult, default_emacs_keybindings,
};
use std::io::BufRead;
use std::io::BufReader;
use std::time::{Duration, Instant};

/// Runtime to use for evaluation
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum Runtime {
    /// Tree-walking evaluator
    Evaluator,
    /// Bytecode VM
    Vm,
    /// Run both and compare results
    #[default]
    Both,
}

/// Debug stages to print
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
enum DebugStage {
    /// Print the parsed AST
    Parser,
    /// Print the typed expression
    Analyzer,
    /// Print the compiled bytecode
    Bytecode,
}

/// Melbi - A safe, fast, embeddable expression language
#[derive(Parser, Debug)]
#[command(name = "melbi")]
#[command(about = "Evaluate Melbi expressions", long_about = None)]
struct Args {
    /// Debug stages to print (comma-separated)
    #[arg(long, value_delimiter = ',')]
    debug: Vec<DebugStage>,

    /// Runtime to use for evaluation
    #[arg(long, default_value = "both")]
    runtime: Runtime,

    /// Disable colored output
    #[arg(long)]
    no_color: bool,

    /// Print execution time for each runtime
    #[arg(long)]
    time: bool,

    /// Expression to evaluate (if not provided, reads from stdin)
    expression: Option<String>,
}

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

    let history_path = dirs::config_dir()
        .expect("Failed to find a suitable config directory")
        .join("melbi/history");
    let history = Box::new(
        FileBackedHistory::with_file(10000, history_path).expect("Failed to initialize history"),
    );

    let validator = Box::new(MelbiValidator);

    let line_editor = Reedline::create()
        .with_highlighter(Box::new(
            Highlighter::new().expect("Failed to initialize highlighter"),
        ))
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

fn interpret_input<'types>(
    type_manager: &'types TypeManager<'types>,
    globals_types: &[(&'types str, &'types Type<'types>)],
    globals_values: &'types [(&'types str, Value<'types, 'types>)],
    input: &str,
    debug: &[DebugStage],
    runtime: Runtime,
    no_color: bool,
    show_time: bool,
) -> Result<()> {
    let config = RenderConfig {
        color: !no_color,
        ..Default::default()
    };
    let render_err = |e: melbi::Error| {
        render_error_to(&e, &mut std::io::stderr(), &config).ok();
    };

    let arena = Bump::new();

    // Parse
    let ast = match parser::parse(&arena, input) {
        Ok(ast) => ast,
        Err(e) => {
            render_err(e.into());
            return Ok(());
        }
    };

    if debug.contains(&DebugStage::Parser) {
        println!("=== Parsed AST ===");
        println!("{:#?}", ast.expr);
        println!();
    }

    // Type check
    let typed = match analyze(type_manager, &arena, &ast, globals_types, &[]) {
        Ok(typed) => typed,
        Err(e) => {
            render_err(e.into());
            return Ok(());
        }
    };

    if debug.contains(&DebugStage::Analyzer) {
        println!("=== Typed Expression ===");
        println!("{:#?}", typed.expr);
        println!("Lambda Instantiations: {:#?}", typed.lambda_instantiations);
        println!();
    }

    // Run with selected runtime(s)
    let run_evaluator = matches!(runtime, Runtime::Evaluator | Runtime::Both);
    let run_vm = matches!(runtime, Runtime::Vm | Runtime::Both);

    let mut eval_result = None;
    let mut eval_duration = None;
    let mut vm_result = None;
    let mut vm_duration = None;

    // Evaluator
    if run_evaluator {
        let mut evaluator = Evaluator::new(
            EvaluatorOptions::default(),
            &arena,
            type_manager,
            &typed,
            globals_values,
            &[],
        );
        let start = Instant::now();
        eval_result = Some(evaluator.eval());
        eval_duration = Some(start.elapsed());
    }

    // VM
    if run_vm {
        let bytecode = match BytecodeCompiler::compile(type_manager, &arena, globals_values, &typed)
        {
            Ok(code) => code,
            Err(e) => {
                eprintln!("Bytecode compilation error: {:?}", e);
                return Ok(());
            }
        };

        if debug.contains(&DebugStage::Bytecode) {
            println!("=== Bytecode ===");
            println!("{:#?}", bytecode);
            println!();
        }

        let result_type = typed.expr.0;
        let start = Instant::now();
        vm_result = Some(
            VM::execute(&arena, &bytecode).map(|raw| Value::from_raw_unchecked(result_type, raw)),
        );
        vm_duration = Some(start.elapsed());
    }

    // Output results
    match (runtime, eval_result, vm_result) {
        (Runtime::Evaluator, Some(Ok(value)), _) => {
            println!("{:?}", value);
        }
        (Runtime::Evaluator, Some(Err(e)), _) => {
            render_err(e.into());
        }
        (Runtime::Vm, _, Some(Ok(value))) => {
            println!("{:?}", value);
        }
        (Runtime::Vm, _, Some(Err(e))) => {
            render_err(e.into());
        }
        (Runtime::Both, Some(eval_res), Some(vm_res)) => {
            match (eval_res, vm_res) {
                (Ok(eval_val), Ok(vm_val)) => {
                    if eval_val == vm_val {
                        println!("{:?}", eval_val);
                    } else {
                        eprintln!("MISMATCH!");
                        eprintln!("  Evaluator: {:?}", eval_val);
                        eprintln!("  VM:        {:?}", vm_val);
                    }
                }
                (Err(e), Ok(vm_val)) => {
                    eprintln!("MISMATCH!");
                    eprintln!("  Evaluator: error");
                    render_err(e.into());
                    eprintln!("  VM:        {:?}", vm_val);
                }
                (Ok(eval_val), Err(e)) => {
                    eprintln!("MISMATCH!");
                    eprintln!("  Evaluator: {:?}", eval_val);
                    eprintln!("  VM:        error");
                    render_err(e.into());
                }
                (Err(eval_e), Err(vm_e)) => {
                    // Both errored - check if same kind of error
                    if eval_e.kind == vm_e.kind {
                        render_err(eval_e.into());
                    } else {
                        eprintln!("MISMATCH (both errors but different)!");
                        eprintln!("  Evaluator:");
                        render_err(eval_e.into());
                        eprintln!("  VM:");
                        render_err(vm_e.into());
                    }
                }
            }
        }
        _ => unreachable!(),
    }

    // Print timing information
    if show_time {
        print_timing(runtime, eval_duration, vm_duration, no_color);
    }

    Ok(())
}

fn format_duration(duration: Duration) -> String {
    let nanos = duration.as_nanos();
    if nanos < 1_000 {
        format!("{}ns", nanos)
    } else if nanos < 1_000_000 {
        format!("{:.2}µs", nanos as f64 / 1_000.0)
    } else if nanos < 1_000_000_000 {
        format!("{:.2}ms", nanos as f64 / 1_000_000.0)
    } else {
        format!("{:.2}s", duration.as_secs_f64())
    }
}

fn print_timing(
    runtime: Runtime,
    eval_duration: Option<Duration>,
    vm_duration: Option<Duration>,
    no_color: bool,
) {
    let dimmed = if no_color {
        Style::new()
    } else {
        Style::new().dimmed()
    };

    match runtime {
        Runtime::Evaluator => {
            if let Some(duration) = eval_duration {
                eprintln!("{}", dimmed.paint(format!("⏱ Evaluator: {}", format_duration(duration))));
            }
        }
        Runtime::Vm => {
            if let Some(duration) = vm_duration {
                eprintln!("{}", dimmed.paint(format!("⏱ VM: {}", format_duration(duration))));
            }
        }
        Runtime::Both => {
            if let (Some(eval_dur), Some(vm_dur)) = (eval_duration, vm_duration) {
                let eval_nanos = eval_dur.as_nanos() as f64;
                let vm_nanos = vm_dur.as_nanos() as f64;

                // Calculate percentage difference (VM relative to Evaluator)
                let diff_percent = if eval_nanos > 0.0 {
                    ((vm_nanos - eval_nanos) / eval_nanos) * 100.0
                } else {
                    0.0
                };

                let diff_str = if diff_percent > 0.0 {
                    format!("+{:.1}%", diff_percent)
                } else {
                    format!("{:.1}%", diff_percent)
                };

                eprintln!(
                    "{}",
                    dimmed.paint(format!(
                        "⏱ Evaluator: {} | VM: {} ({})",
                        format_duration(eval_dur),
                        format_duration(vm_dur),
                        diff_str
                    ))
                );
            }
        }
    }
}

/// Build stdlib and return (globals_types, globals_values) for use with analyze/evaluate
fn build_stdlib<'arena>(
    arena: &'arena Bump,
    type_manager: &'arena TypeManager<'arena>,
) -> (
    &'arena [(&'arena str, &'arena Type<'arena>)],
    &'arena [(&'arena str, Value<'arena, 'arena>)],
) {
    let env_builder = EnvironmentBuilder::new(arena);
    let env_builder = register_stdlib(arena, type_manager, env_builder);
    let globals_values = env_builder
        .build()
        .expect("Environment build should succeed");

    // Convert to types for analyzer
    let globals_types: Vec<(&'arena str, &'arena Type<'arena>)> = globals_values
        .iter()
        .map(|(name, value)| (*name, value.ty))
        .collect();
    let globals_types = arena.alloc_slice_copy(&globals_types);

    (globals_types, globals_values)
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging subscriber
    use tracing_subscriber::{EnvFilter, fmt};

    // Use RUST_LOG environment variable to control log level
    // Default to WARN if not set
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("warn"))
        .unwrap();

    fmt()
        .compact()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .without_time()
        .init();

    // Check if we have a direct expression argument
    if let Some(expr) = args.expression {
        let arena = Bump::new();
        let type_manager = TypeManager::new(&arena);
        let (globals_types, globals_values) = build_stdlib(&arena, type_manager);
        interpret_input(
            type_manager,
            globals_types,
            globals_values,
            &expr,
            &args.debug,
            args.runtime,
            args.no_color,
            args.time,
        )?;
        return Ok(());
    }

    // Otherwise, check if we're in interactive or pipe mode
    let is_interactive = atty::is(atty::Stream::Stdin);

    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);
    let (globals_types, globals_values) = build_stdlib(&arena, type_manager);

    if !is_interactive {
        // Pipe/stdin mode
        let stdin = std::io::stdin();
        let reader = BufReader::new(stdin.lock());

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("Error reading line from stdin: {}", e);
                    return Ok(());
                }
            };

            interpret_input(
                type_manager,
                globals_types,
                globals_values,
                &line,
                &args.debug,
                args.runtime,
                args.no_color,
                args.time,
            )?;
        }
        return Ok(());
    }

    // Interactive REPL mode
    let (mut line_editor, prompt) = setup_reedline();

    let style = Style::new().dimmed();
    println!(
        "🖖 Melbi REPL. {}",
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
                interpret_input(
                    type_manager,
                    globals_types,
                    globals_values,
                    buffer.as_ref(),
                    &args.debug,
                    args.runtime,
                    args.no_color,
                    args.time,
                )?;
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
