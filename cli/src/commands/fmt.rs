//! The `fmt` command - format Melbi files.

use std::path::Path;

use similar::{ChangeTag, TextDiff};
use topiary_core::{FormatterError, Operation, TopiaryQuery};

use crate::cli::FmtArgs;
use crate::common::input::read_file;
use crate::common::CliResult;

const QUERY: &str = include_str!("../../../topiary-queries/queries/melbi.scm");

/// Run the fmt command.
pub fn run(args: FmtArgs, no_color: bool) -> CliResult<()> {
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
                eprintln!("error: {}: {}", file.display(), e);
                has_errors = true;
            }
        }
    }

    if has_errors {
        std::process::exit(1);
    }

    if args.check && needs_formatting {
        std::process::exit(1);
    }

    Ok(())
}

/// Format a single file.
/// Returns Ok(true) if the file needed formatting, Ok(false) if already formatted.
fn format_file(path: &Path, args: &FmtArgs, no_color: bool) -> Result<bool, String> {
    let input = read_file(path)?;
    let formatted = format_source(&input)?;

    if input == formatted {
        return Ok(false);
    }

    if args.write {
        std::fs::write(path, &formatted)
            .map_err(|e| format!("failed to write: {}", e))?;
        println!("formatted {}", path.display());
    } else if args.check {
        println!("{} needs formatting", path.display());
    } else {
        // Default: print diff
        print_diff(path, &input, &formatted, no_color);
    }

    Ok(true)
}

/// Format Melbi source code.
fn format_source(input: &str) -> Result<String, String> {
    let mut output = Vec::new();

    let grammar = topiary_tree_sitter_facade::Language::from(tree_sitter_melbi::LANGUAGE);

    let query = TopiaryQuery::new(&grammar, QUERY).map_err(|e| match e {
        FormatterError::Query(m, e) => match e {
            None => m,
            Some(e) => format!("{m}: {e}"),
        },
        _ => "unknown query error".to_string(),
    })?;

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
    .map_err(|e| match e {
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
    })?;

    let output = String::from_utf8(output).map_err(|e| e.to_string())?;

    // Match input's trailing newline behavior
    if input.ends_with('\n') {
        Ok(output)
    } else {
        Ok(output.trim_end().into())
    }
}

/// Print a unified diff between original and formatted content.
fn print_diff(path: &Path, original: &str, formatted: &str, no_color: bool) {
    let diff = TextDiff::from_lines(original, formatted);

    println!("--- {}", path.display());
    println!("+++ {}", path.display());

    for hunk in diff.unified_diff().iter_hunks() {
        println!("{}", hunk.header());
        for change in hunk.iter_changes() {
            let (sign, color_start, color_end) = match change.tag() {
                ChangeTag::Delete => ("-", if no_color { "" } else { "\x1b[31m" }, if no_color { "" } else { "\x1b[0m" }),
                ChangeTag::Insert => ("+", if no_color { "" } else { "\x1b[32m" }, if no_color { "" } else { "\x1b[0m" }),
                ChangeTag::Equal => (" ", "", ""),
            };
            print!("{}{}{}{}", color_start, sign, change.value(), color_end);
            if !change.value().ends_with('\n') {
                println!();
            }
        }
    }
}
