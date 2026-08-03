use std::string::FromUtf8Error;

use topiary_core::{FormatterError, Operation, TopiaryQuery};

const QUERY: &str = include_str!("../../topiary-queries/queries/melbi.scm");

/// An error that occurred while formatting Melbi source code.
#[derive(thiserror::Error, Debug)]
pub enum FormatError {
    #[error("query error: {message}")]
    Query { message: String },

    /// The formatter's output was not idempotent, i.e. formatting the output
    /// again made further changes. This indicates a bug in the formatter's
    /// query rules, not in the input source code.
    #[error("the formatter's output was not idempotent (this is a bug)")]
    Idempotency,

    #[error("parse error")]
    Parse {
        start_line: usize,
        start_column: usize,
        end_line: usize,
        end_column: usize,
    },

    #[error("internal formatter error: {0}")]
    Internal(String),

    #[error("UTF-8 conversion error")]
    Utf8(#[from] FromUtf8Error),
}

impl From<FormatterError> for FormatError {
    fn from(e: FormatterError) -> Self {
        match e {
            FormatterError::Query(message, source) => FormatError::Query {
                message: match source {
                    None => message,
                    Some(source) => format!("{message}: {source}"),
                },
            },
            FormatterError::Idempotence | FormatterError::IdempotenceParsing(_) => {
                FormatError::Idempotency
            }
            FormatterError::Parsing(err) => FormatError::Parse {
                start_line: err.start_point().row() as usize + 1,
                start_column: err.start_point().column() as usize + 1,
                end_line: err.end_point().row() as usize + 1,
                end_column: err.end_point().column() as usize + 1,
            },
            other => FormatError::Internal(other.to_string()),
        }
    }
}

/// Format Melbi source code.
///
/// A leading shebang line (e.g. `#!/usr/bin/env melbi run`) is preserved as-is
/// and re-attached after formatting the rest of the source.
///
/// # Arguments
///
/// - `input`: Melbi source code to format
/// - `skip_idempotence`: skip check that AST of formatted source is identical to input. This is
///   intended for working around current formatter limitations.
/// - `tolerate_parsing_errors`: whether source code with syntax errors should be accepted or
///   rejected.
///
/// # Examples
///
/// ```
/// # use melbi_fmt::format;
/// let source = "a   + b where{ a = 1, b = 2}";
/// assert_eq!(
///     format(source, false, false).unwrap(),
///     "a + b where { a = 1, b = 2 }"
/// );
/// ```
pub fn format(
    input: &str,
    skip_idempotence: bool,
    tolerate_parsing_errors: bool,
) -> Result<String, FormatError> {
    let (shebang, source) = strip_shebang(input);
    let formatted = format_source(source, skip_idempotence, tolerate_parsing_errors)?;

    Ok(match shebang {
        Some(shebang) => format!("{shebang}{formatted}"),
        None => formatted,
    })
}

/// Strip a shebang line from the input, if present.
///
/// Returns `(Some(shebang_line_with_newline), rest)` if a shebang is found,
/// or `(None, input)` if no shebang is present.
fn strip_shebang(input: &str) -> (Option<&str>, &str) {
    if input.starts_with("#!/") {
        match input.find('\n') {
            Some(pos) => (Some(&input[..=pos]), &input[pos + 1..]),
            None => (Some(input), ""),
        }
    } else {
        (None, input)
    }
}

fn format_source(
    input: &str,
    skip_idempotence: bool,
    tolerate_parsing_errors: bool,
) -> Result<String, FormatError> {
    let mut output = Vec::new();

    let grammar = topiary_tree_sitter_facade::Language::from(tree_sitter_melbi::LANGUAGE);
    let query = TopiaryQuery::new(&grammar, QUERY)?;

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
            skip_idempotence,
            tolerate_parsing_errors,
        },
    )?;

    let output = String::from_utf8(output)?;

    // Final cleanup of result. If we received an input not ending in a newline, also return an
    // output without newline. We do not want to force a newline since we e.g., could be formatting
    // input received from an editor and do not want to insert additional newlines.
    if input.ends_with('\n') {
        Ok(output)
    } else {
        Ok(output.trim_end().into())
    }
}
