//! Unescaping bytes literals.
//!
//! Ported from `core/src/syntax/bytes_literal.rs`, keeping only the *unescape*
//! half, and returning an owned [`Vec<u8>`] for the reason given in
//! [`super::string`].

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Why a bytes literal could not be unescaped.
///
/// `pos` is a byte offset into the literal's contents, as in
/// [`super::string::UnescapeError`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnescapeError {
    /// `\z` — not an escape this language defines.
    InvalidEscape { pos: usize, seq: String },
    /// A non-hex character inside `\xNN`.
    InvalidHexDigit { pos: usize, seq: String },
    /// `\x` or `\x0` — the literal ended before the escape did.
    IncompleteHexEscape { pos: usize },
    /// A literal non-ASCII character. `b"é"` must be written with `\xNN`, since
    /// otherwise the byte string would silently depend on the source encoding.
    NonAsciiCharacter { pos: usize, character: char },
}

impl core::fmt::Display for UnescapeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            UnescapeError::InvalidEscape { pos, seq } => {
                write!(f, "invalid escape sequence '{seq}' at position {pos}")
            }
            UnescapeError::InvalidHexDigit { pos, seq } => {
                write!(f, "invalid hex digit in '{seq}' at position {pos}")
            }
            UnescapeError::IncompleteHexEscape { pos } => {
                write!(f, "incomplete hex escape at position {pos}")
            }
            UnescapeError::NonAsciiCharacter { pos, character } => write!(
                f,
                "non-ASCII character '{character}' at position {pos} (use \\xNN escapes)"
            ),
        }
    }
}

/// Resolve the escape sequences in `input`, the *contents* of a bytes literal
/// with its `b"` prefix and closing quote already stripped.
pub fn unescape_bytes(input: &str) -> Result<Vec<u8>, UnescapeError> {
    // Rejecting non-ASCII up front is what lets the rest of this function treat
    // a `char` as one byte, and makes char offsets and byte offsets agree.
    if let Some(pos) = input.bytes().position(|b| b > 0x7F) {
        let character = input[pos..]
            .chars()
            .next()
            .expect("a non-ASCII byte begins a char boundary or a continuation of one");
        return Err(UnescapeError::NonAsciiCharacter { pos, character });
    }

    let mut output = Vec::with_capacity(input.len());
    let mut chars = input.char_indices().peekable();

    while let Some((pos, ch)) = chars.next() {
        if ch != '\\' {
            output.push(ch as u8);
            continue;
        }

        match chars.next() {
            Some((_, 'n')) => output.push(b'\n'),
            Some((_, 'r')) => output.push(b'\r'),
            Some((_, 't')) => output.push(b'\t'),
            Some((_, '0')) => output.push(b'\0'),
            Some((_, '\\')) => output.push(b'\\'),
            Some((_, '"')) => output.push(b'"'),
            Some((_, '\'')) => output.push(b'\''),
            // Line continuation: the backslash and the newline both vanish.
            Some((_, '\n')) => {}
            Some((_, '\r')) => {
                // Accept a CRLF line ending as one continuation.
                if chars.peek().map(|&(_, c)| c) == Some('\n') {
                    chars.next();
                }
            }
            Some((_, 'x')) => {
                let (Some((_, high_char)), Some((_, low_char))) = (chars.next(), chars.next())
                else {
                    return Err(UnescapeError::IncompleteHexEscape { pos });
                };
                let bad_digit = || UnescapeError::InvalidHexDigit {
                    pos,
                    seq: format!("\\x{high_char}{low_char}"),
                };
                let high = high_char.to_digit(16).ok_or_else(bad_digit)?;
                let low = low_char.to_digit(16).ok_or_else(bad_digit)?;
                output.push(((high << 4) | low) as u8);
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
