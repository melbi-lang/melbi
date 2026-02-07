//! The `fmt` command - format Melbi files.

use std::process::ExitCode;

use nu_ansi_term::Color;
use similar::{ChangeTag, TextDiff};
use topiary_core::{FormatterError, Operation, TopiaryQuery};

use crate::cli::FmtArgs;
use crate::common::input::{is_stdin, read_input, strip_shebang};

const QUERY: &str = include_str!("../../../topiary-queries/queries/melbi.scm");

/// Run the fmt command.
pub fn run(args: FmtArgs, no_color: bool) -> ExitCode {
    let mut has_errors = false;
    let mut needs_formatting = false;

    for file in &args.files {
        match format_file(file, &args, no_color) {
            Ok(changed) => {
                if changed {
                    needs_formatting = true;
                }
            }
            Err(e) => {
                // TODO: Do not use strings as error messages. Reuse/update Melbi types.
                if !args.quiet {
                    eprintln!("error: {}: {}", file, e);
                }
                has_errors = true;
            }
        }
    }

    if has_errors {
        return ExitCode::FAILURE;
    }

    if args.check && needs_formatting {
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

/// Format a single file or stdin.
/// Returns Ok(true) if the input needed formatting, Ok(false) if already formatted.
fn format_file(path: &str, args: &FmtArgs, no_color: bool) -> Result<bool, String> {
    let from_stdin = is_stdin(path);

    // --write is incompatible with stdin
    if args.write && from_stdin {
        return Err("cannot use --write with stdin".to_string());
    }

    let (input, display_name) = read_input(path)?;

    // Strip shebang line if present, we'll re-attach it after formatting
    let (shebang, source) = strip_shebang(&input);
    let formatted_source = format_source(source)?;

    // Re-attach shebang if present
    let formatted = match shebang {
        Some(shebang) => format!("{}{}", shebang, formatted_source),
        None => formatted_source,
    };

    if input == formatted {
        return Ok(false);
    }

    if args.write {
        std::fs::write(path, &formatted).map_err(|e| format!("failed to write: {}", e))?;
        if !args.quiet {
            println!("formatted {}", display_name);
        }
    } else if args.quiet {
        // quiet mode without write - no output
    } else if args.check {
        println!("{} needs formatting", display_name);
    } else if from_stdin {
        // For stdin without --write or --check, just print formatted output
        print!("{}", formatted);
    } else {
        // Default for files: print diff
        print_diff(&display_name, &input, &formatted, no_color);
    }

    Ok(true)
}

/// Format a `FormatterError` into a human-readable error message.
fn format_formatter_error(e: FormatterError) -> String {
    match e {
        FormatterError::Query(m, e) => match e {
            None => m,
            Some(e) => format!("{m}: {e}"),
        },
        FormatterError::Idempotence => "idempotency check failed".to_string(),
        FormatterError::Parsing {
            start_line,
            start_column,
            ..
        } => format!("parse error at {}:{}", start_line, start_column),
        _ => "unknown formatter error".to_string(),
    }
}

/// Format Melbi source code.
fn format_source(input: &str) -> Result<String, String> {
    let mut output = Vec::new();

    let grammar = topiary_tree_sitter_facade::Language::from(tree_sitter_melbi::LANGUAGE);

    let query = TopiaryQuery::new(&grammar, QUERY).map_err(format_formatter_error)?;

    let language = topiary_core::Language {
        name: "melbi".to_string(),
        indent: Some("    ".to_string()),
        grammar,
        query,
    };

    topiary_core::formatter(
        &mut input.as_bytes(),
        &mut output,
        &language,
        Operation::Format {
            skip_idempotence: false,
            tolerate_parsing_errors: false,
        },
    )
    .map_err(format_formatter_error)?;

    let output = String::from_utf8(output).map_err(|e| e.to_string())?;

    // Match input's trailing newline behavior
    if input.ends_with('\n') {
        Ok(output)
    } else {
        Ok(output.trim_end().into())
    }
}

/// Print a unified diff between original and formatted content.
fn print_diff(name: &str, original: &str, formatted: &str, no_color: bool) {
    let diff = TextDiff::from_lines(original, formatted);

    println!("--- {}", name);
    println!("+++ {}", name);

    for hunk in diff.unified_diff().iter_hunks() {
        println!("{}", hunk.header());
        for change in hunk.iter_changes() {
            let sign = match change.tag() {
                ChangeTag::Delete => "-",
                ChangeTag::Insert => "+",
                ChangeTag::Equal => " ",
            };
            let line = format!("{}{}", sign, change.value());
            let colored = if no_color {
                line
            } else {
                match change.tag() {
                    ChangeTag::Delete => Color::Red.paint(&line).to_string(),
                    ChangeTag::Insert => Color::Green.paint(&line).to_string(),
                    ChangeTag::Equal => line,
                }
            };
            print!("{}", colored);
            if !change.value().ends_with('\n') {
                println!();
            }
        }
    }
}
