/// Bytes literal escaping and unescaping for Melbi syntax.
///
/// This module provides utilities for converting between:
/// - Runtime bytes (e.g., `[104, 101, 108, 108, 111]` for "hello")
/// - Melbi source code bytes literals (e.g., `b"hello"` or `b"\x68\x65\x6c\x6c\x6f"`)
use alloc::string::ToString;
use core::fmt;

use bumpalo::Bump;

use crate::{String, format};

/// Style for quoting bytes literals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QuoteStyle {
    /// Always use single quotes: `b'...'`
    AlwaysSingle,
    /// Always use double quotes: `b"..."`
    AlwaysDouble,
    /// Prefer single quotes, use double if content contains single quote
    PreferSingle,
    /// Prefer double quotes, use single if content contains double quote (but not single)
    #[default]
    PreferDouble,
}

/// Errors that can occur when unescaping bytes literals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnescapeError {
    /// Invalid escape sequence (e.g., `\a`, `\z`)
    InvalidEscape { pos: usize, seq: String },
    /// Invalid hex digit in `\xNN` escape
    InvalidHexDigit { pos: usize, seq: String },
    /// Incomplete hex escape (e.g., `\x`, `\x0`)
    IncompleteHexEscape { pos: usize },
    /// Non-ASCII character in bytes literal
    NonAsciiCharacter { pos: usize, character: char },
}

impl fmt::Display for UnescapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UnescapeError::InvalidEscape { pos, seq } => {
                write!(f, "invalid escape sequence '{}' at position {}", seq, pos)
            }
            UnescapeError::InvalidHexDigit { pos, seq } => {
                write!(f, "invalid hex digit in '{}' at position {}", seq, pos)
            }
            UnescapeError::IncompleteHexEscape { pos } => {
                write!(f, "incomplete hex escape at position {}", pos)
            }
            UnescapeError::NonAsciiCharacter { pos, character } => {
                write!(
                    f,
                    "non-ASCII character '{}' at position {} (use \\xNN escapes)",
                    character, pos
                )
            }
        }
    }
}

/// Escape bytes for Melbi bytes literal representation.
///
/// This function formats bytes to be human-readable when possible:
/// - Printable ASCII characters (0x20-0x7E) are shown directly
/// - Common escape sequences use backslash notation (`\n`, `\r`, `\t`, `\"`, `\\`)
/// - Non-printable bytes use hex notation (`\xNN`)
///
/// The quote style determines which quote character to use and which to escape.
///
/// # Example
///
/// ```ignore
/// use melbi_core::syntax::bytes_literal::{escape_bytes, QuoteStyle};
///
/// let bytes = b"hello\nworld";
/// let mut output = String::new();
/// escape_bytes(&mut output, bytes, QuoteStyle::PreferDouble).unwrap();
/// assert_eq!(output, r#"b"hello\nworld""#);
/// ```
pub fn escape_bytes(f: &mut impl fmt::Write, bytes: &[u8], style: QuoteStyle) -> fmt::Result {
    // Determine which quote character to use
    let (quote_char, needs_escape) = match style {
        QuoteStyle::AlwaysSingle => ('\'', b'\''),
        QuoteStyle::AlwaysDouble => ('"', b'"'),
        QuoteStyle::PreferSingle => {
            // Use double quotes if contains single quote but not double quote
            if bytes.contains(&b'\'') && !bytes.contains(&b'"') {
                ('"', b'"')
            } else {
                ('\'', b'\'')
            }
        }
        QuoteStyle::PreferDouble => {
            // Use single quotes if contains double quote but not single quote
            if bytes.contains(&b'"') && !bytes.contains(&b'\'') {
                ('\'', b'\'')
            } else {
                ('"', b'"')
            }
        }
    };

    write!(f, "b{}", quote_char)?;

    for &byte in bytes {
        if byte == needs_escape {
            write!(f, "\\{}", quote_char)?;
        } else {
            match byte {
                b'\\' => write!(f, "\\\\")?,
                b'\n' => write!(f, "\\n")?,
                b'\r' => write!(f, "\\r")?,
                b'\t' => write!(f, "\\t")?,
                b'\0' => write!(f, "\\x00")?, // Use hex for null to distinguish from \0 (future)
                // Printable ASCII characters (excluding control characters)
                0x20..=0x7E => write!(f, "{}", byte as char)?,
                // Non-printable bytes as hex
                _ => write!(f, "\\x{:02x}", byte)?,
            }
        }
    }

    write!(f, "{}", quote_char)?;
    Ok(())
}

/// Unescape a bytes literal string to actual bytes.
///
/// Takes the content between the quotes (e.g., for `b"hello"`, pass `"hello"`)
/// and converts escape sequences to their byte values.
///
/// Supported escape sequences:
/// - `\n` - newline (0x0A)
/// - `\r` - carriage return (0x0D)
/// - `\t` - tab (0x09)
/// - `\\` - backslash (0x5C)
/// - `\"` - double quote (0x22)
/// - `\'` - single quote (0x27)
/// - `\0` - null byte (0x00)
/// - `\xNN` - hex byte (exactly 2 hex digits)
/// - `\` + newline - line continuation (removes backslash and newline only, preserves following whitespace)
///
/// Only ASCII characters (0x00-0x7F) are allowed in the input.
///
/// **Note**: `\0` and line continuation are supported by this function but require
/// grammar updates for parser integration.
///
/// # Fast path
///
/// If the input contains no backslashes and no non-ASCII characters,
/// returns a direct reference to the input without allocation.
///
/// # Example
///
/// ```ignore
/// use bumpalo::Bump;
/// use melbi_core::syntax::bytes_literal::unescape_bytes;
///
/// let arena = Bump::new();
/// let bytes = unescape_bytes(&arena, r#"hello\x20world"#).unwrap();
/// assert_eq!(bytes, b"hello world");
/// ```
pub fn unescape_bytes<'a>(arena: &'a Bump, input: &'a str) -> Result<&'a [u8], UnescapeError> {
    let input_bytes = input.as_bytes();

    // Fast path: check for non-ASCII first
    if let Some(pos) = input_bytes.iter().position(|&b| b > 0x7F) {
        let character = input[pos..].chars().next().unwrap();
        return Err(UnescapeError::NonAsciiCharacter { pos, character });
    }

    // Fast path: no escapes means we can return input directly (zero-copy!)
    if !input_bytes.contains(&b'\\') {
        return Ok(input.as_bytes());
    }

    // Slow path: process escape sequences
    let output = arena.alloc_slice_fill_default::<u8>(input.len());
    let mut write_pos = 0;
    let mut chars = input.chars().enumerate().peekable();

    while let Some((pos, ch)) = chars.next() {
        if ch != '\\' {
            // Regular ASCII character
            output[write_pos] = ch as u8;
            write_pos += 1;
            continue;
        }

        // We have a backslash - process escape sequence
        let escape_start = pos;
        match chars.next() {
            Some((_, 'n')) => {
                output[write_pos] = b'\n';
                write_pos += 1;
            }
            Some((_, 'r')) => {
                output[write_pos] = b'\r';
                write_pos += 1;
            }
            Some((_, 't')) => {
                output[write_pos] = b'\t';
                write_pos += 1;
            }
            Some((_, '\\')) => {
                output[write_pos] = b'\\';
                write_pos += 1;
            }
            Some((_, '"')) => {
                output[write_pos] = b'"';
                write_pos += 1;
            }
            Some((_, '\'')) => {
                output[write_pos] = b'\'';
                write_pos += 1;
            }
            Some((_, '0')) => {
                // Null byte escape: \0
                output[write_pos] = b'\0';
                write_pos += 1;
            }
            Some((_, '\n')) => {
                // Line continuation: backslash followed by newline
                // Just skip both (don't write anything to output)
                // Whitespace on the next line is preserved
            }
            Some((_, '\r')) => {
                // Check if this is \r\n (Windows line ending)
                if chars.peek().map(|(_, c)| *c) == Some('\n') {
                    chars.next(); // Consume the \n
                }
                // Line continuation with \r or \r\n - skip (don't write anything)
            }
            Some((_, 'x')) => {
                // Hex escape: \xNN
                let hex_start = pos;

                // Get first hex digit
                let (_, d1) = chars
                    .next()
                    .ok_or(UnescapeError::IncompleteHexEscape { pos: hex_start })?;

                // Get second hex digit
                let (_, d2) = chars
                    .next()
                    .ok_or(UnescapeError::IncompleteHexEscape { pos: hex_start })?;

                // Parse hex digits
                let high = d1
                    .to_digit(16)
                    .ok_or_else(|| UnescapeError::InvalidHexDigit {
                        pos: hex_start,
                        seq: format!("\\x{}{}", d1, d2),
                    })?;

                let low = d2
                    .to_digit(16)
                    .ok_or_else(|| UnescapeError::InvalidHexDigit {
                        pos: hex_start,
                        seq: format!("\\x{}{}", d1, d2),
                    })?;

                output[write_pos] = ((high << 4) | low) as u8;
                write_pos += 1;
            }
            Some((_, other)) => {
                // Invalid escape sequence
                return Err(UnescapeError::InvalidEscape {
                    pos: escape_start,
                    seq: format!("\\{}", other),
                });
            }
            None => {
                // Backslash at end of string
                return Err(UnescapeError::InvalidEscape {
                    pos: escape_start,
                    seq: "\\".to_string(),
                });
            }
        }
    }

    Ok(&output[..write_pos])
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to escape bytes to a String
    fn escape_to_string(bytes: &[u8], style: QuoteStyle) -> String {
        let mut output = String::new();
        escape_bytes(&mut output, bytes, style).unwrap();
        output
    }

    // Helper to unescape bytes
    fn unescape(input: &str) -> Result<Vec<u8>, UnescapeError> {
        let arena = Bump::new();
        unescape_bytes(&arena, input).map(|s| s.to_vec())
    }

    // ========================================================================
    // escape_bytes tests
    // ========================================================================

    #[test]
    fn escape_empty() {
        assert_eq!(escape_to_string(b"", QuoteStyle::PreferDouble), r#"b"""#);
    }

    #[test]
    fn escape_simple_ascii() {
        assert_eq!(
            escape_to_string(b"hello", QuoteStyle::PreferDouble),
            r#"b"hello""#
        );
    }

    #[test]
    fn escape_with_spaces() {
        assert_eq!(
            escape_to_string(b"hello world", QuoteStyle::PreferDouble),
            r#"b"hello world""#
        );
    }

    #[test]
    fn escape_control_characters() {
        assert_eq!(
            escape_to_string(b"\n\r\t", QuoteStyle::PreferDouble),
            r#"b"\n\r\t""#
        );
    }

    #[test]
    fn escape_backslash() {
        assert_eq!(
            escape_to_string(b"\\", QuoteStyle::PreferDouble),
            r#"b"\\""#
        );
    }

    #[test]
    fn escape_quotes_prefer_double() {
        // Single quote doesn't need escaping in double quotes
        assert_eq!(
            escape_to_string(b"it's", QuoteStyle::PreferDouble),
            r#"b"it's""#
        );

        // Double quote needs escaping in double quotes normally
        assert_eq!(
            escape_to_string(b"say \"hi\"", QuoteStyle::PreferDouble),
            r#"b'say "hi"'"# // Switches to single quotes!
        );

        // Both quotes present - stick with double, escape double quote
        assert_eq!(
            escape_to_string(b"\"'\"", QuoteStyle::PreferDouble),
            r#"b"\"'\"""#
        );
    }

    #[test]
    fn escape_quotes_prefer_single() {
        // Double quote doesn't need escaping in single quotes
        assert_eq!(
            escape_to_string(b"say \"hi\"", QuoteStyle::PreferSingle),
            r#"b'say "hi"'"#
        );

        // Single quote needs escaping in single quotes normally
        assert_eq!(
            escape_to_string(b"it's", QuoteStyle::PreferSingle),
            r#"b"it's""# // Switches to double quotes!
        );
    }

    #[test]
    fn escape_quotes_always_double() {
        assert_eq!(
            escape_to_string(b"it's", QuoteStyle::AlwaysDouble),
            r#"b"it's""#
        );
        assert_eq!(
            escape_to_string(b"say \"hi\"", QuoteStyle::AlwaysDouble),
            r#"b"say \"hi\"""#
        );
    }

    #[test]
    fn escape_quotes_always_single() {
        assert_eq!(
            escape_to_string(b"it's", QuoteStyle::AlwaysSingle),
            r#"b'it\'s'"#
        );
        assert_eq!(
            escape_to_string(b"say \"hi\"", QuoteStyle::AlwaysSingle),
            r#"b'say "hi"'"#
        );
    }

    #[test]
    fn escape_null_byte() {
        assert_eq!(
            escape_to_string(b"\x00", QuoteStyle::PreferDouble),
            r#"b"\x00""#
        );
    }

    #[test]
    fn escape_non_printable() {
        assert_eq!(
            escape_to_string(b"\x00\xff\x1f", QuoteStyle::PreferDouble),
            r#"b"\x00\xff\x1f""#
        );
    }

    #[test]
    fn escape_mixed() {
        assert_eq!(
            escape_to_string(b"Hello\x20World\n", QuoteStyle::PreferDouble),
            r#"b"Hello World\n""#
        );
    }

    // ========================================================================
    // unescape_bytes tests
    // ========================================================================

    #[test]
    fn unescape_empty() {
        assert_eq!(unescape("").unwrap(), b"");
    }

    #[test]
    fn unescape_simple_ascii_fast_path() {
        // No escapes, should use fast path
        assert_eq!(unescape("hello").unwrap(), b"hello");
        assert_eq!(unescape("hello world").unwrap(), b"hello world");
    }

    #[test]
    fn unescape_standard_escapes() {
        assert_eq!(unescape(r#"\n"#).unwrap(), b"\n");
        assert_eq!(unescape(r#"\r"#).unwrap(), b"\r");
        assert_eq!(unescape(r#"\t"#).unwrap(), b"\t");
        assert_eq!(unescape(r#"\\"#).unwrap(), b"\\");
        assert_eq!(unescape(r#"\""#).unwrap(), b"\"");
        assert_eq!(unescape(r#"\'"#).unwrap(), b"'");
    }

    #[test]
    fn unescape_multiple_escapes() {
        assert_eq!(unescape(r#"\n\r\t"#).unwrap(), b"\n\r\t");
        assert_eq!(unescape(r#"\\\"\'"#).unwrap(), b"\\\"'");
    }

    #[test]
    fn unescape_hex_escapes() {
        assert_eq!(unescape(r#"\x00"#).unwrap(), b"\x00");
        assert_eq!(unescape(r#"\xff"#).unwrap(), b"\xff");
        assert_eq!(unescape(r#"\x42"#).unwrap(), b"B");
        assert_eq!(unescape(r#"\x00\xff\x42"#).unwrap(), b"\x00\xff\x42");
    }

    #[test]
    fn unescape_hex_case_insensitive() {
        assert_eq!(unescape(r#"\xFF"#).unwrap(), b"\xff");
        assert_eq!(unescape(r#"\xAb"#).unwrap(), b"\xab");
        assert_eq!(unescape(r#"\xCd"#).unwrap(), b"\xcd");
    }

    #[test]
    fn unescape_mixed() {
        assert_eq!(unescape(r#"Hello\x20World"#).unwrap(), b"Hello World");
        assert_eq!(unescape(r#"line1\nline2"#).unwrap(), b"line1\nline2");
        assert_eq!(
            unescape(r#"tab\there\nand\\slash"#).unwrap(),
            b"tab\there\nand\\slash"
        );
    }

    // ========================================================================
    // Error cases
    // ========================================================================

    #[test]
    fn unescape_invalid_escape() {
        assert_eq!(
            unescape(r#"\a"#),
            Err(UnescapeError::InvalidEscape {
                pos: 0,
                seq: "\\a".to_string()
            })
        );
        assert_eq!(
            unescape(r#"\z"#),
            Err(UnescapeError::InvalidEscape {
                pos: 0,
                seq: "\\z".to_string()
            })
        );
        assert_eq!(
            unescape(r#"hello\b"#),
            Err(UnescapeError::InvalidEscape {
                pos: 5,
                seq: "\\b".to_string()
            })
        );
    }

    #[test]
    fn unescape_invalid_hex_digit() {
        assert_eq!(
            unescape(r#"\xGG"#),
            Err(UnescapeError::InvalidHexDigit {
                pos: 0,
                seq: "\\xGG".to_string()
            })
        );
        assert_eq!(
            unescape(r#"\xZ0"#),
            Err(UnescapeError::InvalidHexDigit {
                pos: 0,
                seq: "\\xZ0".to_string()
            })
        );
    }

    #[test]
    fn unescape_incomplete_hex_escape() {
        assert_eq!(
            unescape(r#"\x"#),
            Err(UnescapeError::IncompleteHexEscape { pos: 0 })
        );
        assert_eq!(
            unescape(r#"\x0"#),
            Err(UnescapeError::IncompleteHexEscape { pos: 0 })
        );
        assert_eq!(
            unescape(r#"hello\x"#),
            Err(UnescapeError::IncompleteHexEscape { pos: 5 })
        );
    }

    #[test]
    fn unescape_backslash_at_end() {
        assert_eq!(
            unescape(r#"\"#),
            Err(UnescapeError::InvalidEscape {
                pos: 0,
                seq: "\\".to_string()
            })
        );
    }

    // ========================================================================
    // Round-trip tests
    // ========================================================================

    #[test]
    fn roundtrip_simple() {
        let original = b"hello world";
        let escaped = escape_to_string(original, QuoteStyle::PreferDouble);
        // Strip b"..." wrapper
        let inner = &escaped[2..escaped.len() - 1];
        let unescaped = unescape(inner).unwrap();
        assert_eq!(&unescaped[..], original);
    }

    #[test]
    fn roundtrip_with_escapes() {
        let original = b"line1\nline2\ttab";
        let escaped = escape_to_string(original, QuoteStyle::PreferDouble);
        let inner = &escaped[2..escaped.len() - 1];
        let unescaped = unescape(inner).unwrap();
        assert_eq!(&unescaped[..], original);
    }

    #[test]
    fn roundtrip_binary() {
        let original = b"\x00\x01\x02\xff\xfe\xfd";
        let escaped = escape_to_string(original, QuoteStyle::PreferDouble);
        let inner = &escaped[2..escaped.len() - 1];
        let unescaped = unescape(inner).unwrap();
        assert_eq!(&unescaped[..], original);
    }

    // ========================================================================
    // Future features (ignored tests)
    // ========================================================================

    #[test]
    fn unescape_null_escape() {
        // Note: Grammar doesn't support \0 yet, but unescape function does
        assert_eq!(unescape(r#"\0"#).unwrap(), b"\x00");
    }

    #[test]
    fn unescape_line_continuation_simple() {
        // Note: Grammar doesn't support line continuation yet, but unescape function does
        assert_eq!(unescape("hello\\\nworld").unwrap(), b"helloworld");
    }

    #[test]
    fn unescape_line_continuation_preserves_whitespace() {
        // Note: Grammar doesn't support line continuation yet, but unescape function does
        // Backslash-newline removes only the backslash and newline
        // Whitespace on the next line is preserved
        assert_eq!(unescape("hello\\\n    world").unwrap(), b"hello    world");
    }

    #[test]
    fn unescape_line_continuation_windows() {
        assert_eq!(unescape("hello\\\r\n world").unwrap(), b"hello world");
    }

    #[test]
    fn reject_non_ascii_simple() {
        assert_eq!(
            unescape("café"),
            Err(UnescapeError::NonAsciiCharacter {
                pos: 3,
                character: 'é'
            })
        );
    }

    #[test]
    fn reject_non_ascii_emoji() {
        assert_eq!(
            unescape("hello 🌍"),
            Err(UnescapeError::NonAsciiCharacter {
                pos: 6,
                character: '🌍'
            })
        );
    }

    #[test]
    fn reject_non_ascii_mixed() {
        assert_eq!(
            unescape("melbi 🖖"),
            Err(UnescapeError::NonAsciiCharacter {
                pos: 6,
                character: '🖖'
            })
        );
    }
}
