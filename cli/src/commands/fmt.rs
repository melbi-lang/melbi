//! The `fmt` command - format Melbi files.

use std::process::ExitCode;

use melbi_fmt::FormatError;
use nu_ansi_term::Color;
use similar::{ChangeTag, TextDiff};

use crate::cli::FmtArgs;
use crate::common::input::{is_stdin, read_input};

/// Run the fmt command.
#[must_use]
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
                    eprintln!("error: {file}: {e}");
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

    let formatted = melbi_fmt::format(&input, false, false).map_err(describe_error)?;

    if input == formatted {
        return Ok(false);
    }

    if args.write {
        std::fs::write(path, &formatted).map_err(|e| format!("failed to write: {e}"))?;
        if !args.quiet {
            println!("formatted {display_name}");
        }
    } else if args.quiet {
        // quiet mode without write - no output
    } else if args.check {
        println!("{display_name} needs formatting");
    } else if from_stdin {
        // For stdin without --write or --check, just print formatted output
        print!("{formatted}");
    } else {
        // Default for files: print diff
        print_diff(&display_name, &input, &formatted, no_color);
    }

    Ok(true)
}

/// Format a `melbi_fmt::FormatError` into a human-readable error message.
fn describe_error(e: FormatError) -> String {
    match e {
        FormatError::Parse {
            start_line,
            start_column,
            ..
        } => format!("parse error at {start_line}:{start_column}"),
        other => other.to_string(),
    }
}

/// Print a unified diff between original and formatted content.
fn print_diff(name: &str, original: &str, formatted: &str, no_color: bool) {
    let diff = TextDiff::from_lines(original, formatted);

    println!("--- {name}");
    println!("+++ {name}");

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
            print!("{colored}");
            if !change.value().ends_with('\n') {
                println!();
            }
        }
    }
}
