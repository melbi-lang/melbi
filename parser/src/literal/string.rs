//! Unescaping string literals.
//!
//! Ported from `core/src/syntax/string_literal.rs`. Only the *unescape* half
//! moved: escaping is the formatter's business and stays in `melbi-core` until
//! that migrates.
//!
//! Unlike the original, this returns an owned [`String`] rather than writing
//! into an arena, so it knows nothing about [`TreeBuilder`]; the caller interns
//! the result with `builder.alloc_str(…)`. That costs the original's zero-copy
//! fast path for escape-free literals — `alloc_str` copies regardless — and buys
//! a function that is testable on its own and reusable by anything lexical.
//!
//! [`TreeBuilder`]: crate::TreeBuilder

use alloc::format;
use alloc::string::{String, ToString};

/// Why a string literal could not be unescaped.
///
/// Every variant carries `pos`, a byte offset **into the literal's contents** —
/// not into the source. The caller adds the offset of the opening quote to
/// point at the real location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnescapeError {
    /// `\q` — not an escape this language defines.
    InvalidEscape { pos: usize, seq: String },
    /// A non-hex character inside `\uNNNN` or `\UNNNNNNNN`.
    InvalidHexDigit { pos: usize, seq: String },
    /// `\u12` — the literal ended before the escape did.
    IncompleteUnicodeEscape {
        pos: usize,
        expected: usize,
        got: usize,
    },
    /// `\uD800` — a surrogate or out-of-range value, which is not a `char`.
    InvalidUnicodeScalar { pos: usize, value: u32 },
    /// A lone `{` or `}` in a format string, where they must be doubled.
    UnpairedBrace { pos: usize, brace: char },
}

impl core::fmt::Display for UnescapeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidEscape { pos, seq } => {
                write!(f, "invalid escape sequence '{seq}' at position {pos}")
            }
            Self::InvalidHexDigit { pos, seq } => {
                write!(f, "invalid hex digit in '{seq}' at position {pos}")
            }
            Self::IncompleteUnicodeEscape { pos, expected, got } => write!(
                f,
                "incomplete Unicode escape at position {pos}: expected {expected} digits, got {got}"
            ),
            Self::InvalidUnicodeScalar { pos, value } => {
                write!(
                    f,
                    "invalid Unicode scalar value U+{value:X} at position {pos}"
                )
            }
            Self::UnpairedBrace { pos, brace } => write!(
                f,
                "unpaired '{brace}' in format string at position {pos} (must be '{{{{' or '}}}}')"
            ),
        }
    }
}

/// Resolve the escape sequences in `input`, the *contents* of a string literal
/// with its quotes already stripped.
///
/// When `is_format_string`, a `{` or `}` must be doubled and collapses to one;
/// a lone brace is [`UnpairedBrace`]. That case only ever sees the literal text
/// *between* the holes of an `f"…"`, since the grammar has already split the
/// interpolated expressions out.
///
/// [`UnpairedBrace`]: UnescapeError::UnpairedBrace
pub fn unescape_string(input: &str, is_format_string: bool) -> Result<String, UnescapeError> {
    // Every escape shrinks, and a doubled brace halves, so the input length is
    // an upper bound on the output.
    let mut output = String::with_capacity(input.len());
    let mut chars = input.char_indices().peekable();

    while let Some((pos, ch)) = chars.next() {
        if is_format_string && (ch == '{' || ch == '}') {
            match chars.peek().map(|&(_, c)| c) {
                Some(next) if next == ch => {
                    chars.next();
                    output.push(ch);
                    continue;
                }
                _ => return Err(UnescapeError::UnpairedBrace { pos, brace: ch }),
            }
        }

        if ch != '\\' {
            output.push(ch);
            continue;
        }

        match chars.next() {
            Some((_, 'n')) => output.push('\n'),
            Some((_, 'r')) => output.push('\r'),
            Some((_, 't')) => output.push('\t'),
            Some((_, '0')) => output.push('\0'),
            Some((_, '\\')) => output.push('\\'),
            Some((_, '"')) => output.push('"'),
            Some((_, '\'')) => output.push('\''),
            // Line continuation: both the backslash and the newline vanish, and
            // the next line's leading whitespace is kept as written.
            //
            // TODO: unlike the bytes literals, this does not accept `\r` or
            // `\r\n`, so a CRLF source file cannot use a line continuation. That
            // asymmetry is inherited from the original and looks unintended;
            // left alone here because changing it is a language decision.
            Some((_, '\n')) => {}
            Some((escape_pos, marker @ ('u' | 'U'))) => {
                let expected = if marker == 'u' { 4 } else { 8 };
                let value = read_hex_digits(&mut chars, expected, pos, escape_pos, marker)?;
                let ch = char::from_u32(value)
                    .ok_or(UnescapeError::InvalidUnicodeScalar { pos, value })?;
                output.push(ch);
            }
            Some((_, other)) => {
                return Err(UnescapeError::InvalidEscape {
                    pos,
                    seq: format!("\\{other}"),
                });
            }
            None => {
                return Err(UnescapeError::InvalidEscape {
                    pos,
                    seq: "\\".to_string(),
                });
            }
        }
    }

    Ok(output)
}

/// Read exactly `expected` hex digits of a `\u`/`\U` escape.
///
/// `escape_pos` is the offset of the marker itself, which is what the original
/// reports for a bad digit, while `pos` — the backslash — is what it reports for
/// a truncated escape. Preserved as-is.
fn read_hex_digits(
    chars: &mut core::iter::Peekable<core::str::CharIndices<'_>>,
    expected: usize,
    pos: usize,
    escape_pos: usize,
    marker: char,
) -> Result<u32, UnescapeError> {
    let mut value = 0u32;

    for got in 0..expected {
        let Some((_, ch)) = chars.next() else {
            return Err(UnescapeError::IncompleteUnicodeEscape { pos, expected, got });
        };
        let Some(digit) = ch.to_digit(16) else {
            return Err(UnescapeError::InvalidHexDigit {
                pos: escape_pos + 1,
                seq: format!("\\{marker}{ch}"),
            });
        };
        value = (value << 4) | digit;
    }

    Ok(value)
}
