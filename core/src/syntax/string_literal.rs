use alloc::string::ToString;
use core::fmt;

use bumpalo::Bump;

/// String literal escaping and unescaping for Melbi syntax.
///
/// This module provides utilities for converting between:
/// - Runtime strings (e.g., "hello\n" with actual newline character)
/// - Melbi source code string literals (e.g., "hello\n" with backslash-n sequence)
use crate::{String, format};

/// Controls which quote style to use when escaping strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QuoteStyle {
    /// Always use single quotes: `'...'`
    AlwaysSingle,
    /// Always use double quotes: `"..."`
    AlwaysDouble,
    /// Prefer single quotes, use double if string contains single quotes but not double
    PreferSingle,
    /// Prefer double quotes, use single if string contains double quotes but not single
    #[default]
    PreferDouble,
}

/// Errors that can occur when unescaping string literals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnescapeError {
    /// Invalid escape sequence (e.g., `\q`)
    InvalidEscape { pos: usize, seq: String },
    /// Invalid hex digit in Unicode escape
    InvalidHexDigit { pos: usize, seq: String },
    /// Incomplete Unicode escape (not enough digits)
    IncompleteUnicodeEscape {
        pos: usize,
        expected: usize,
        got: usize,
    },
    /// Invalid Unicode scalar value
    InvalidUnicodeScalar { pos: usize, value: u32 },
    /// Unpaired brace in format string (must be {{ or }})
    UnpairedBrace { pos: usize, brace: char },
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
            UnescapeError::IncompleteUnicodeEscape { pos, expected, got } => {
                write!(
                    f,
                    "incomplete Unicode escape at position {}: expected {} digits, got {}",
                    pos, expected, got
                )
            }
            UnescapeError::InvalidUnicodeScalar { pos, value } => {
                write!(
                    f,
                    "invalid Unicode scalar value U+{:X} at position {}",
                    value, pos
                )
            }
            UnescapeError::UnpairedBrace { pos, brace } => {
                write!(
                    f,
                    "unpaired '{}' in format string at position {} (must be '{{{{' or '}}}}')",
                    brace, pos
                )
            }
        }
    }
}

/// Escape special characters in strings for Melbi string literals.
///
/// Converts runtime strings to their source code representation by escaping:
/// - `"` → `\"` (or `'` → `\'` depending on quote style)
/// - `\` → `\\`
/// - `\n` → `\n`
/// - `\r` → `\r`
/// - `\t` → `\t`
/// - `\0` → `\0`
/// - Control characters → `\uNNNN`
///
/// The function intelligently chooses quotes based on the `QuoteStyle`:
/// - `PreferDouble`: Uses `"..."` unless the string contains `"` but not `'`
/// - `PreferSingle`: Uses `'...'` unless the string contains `'` but not `"`
/// - `AlwaysDouble`: Always uses `"..."`
/// - `AlwaysSingle`: Always uses `'...'`
///
/// # Example
///
/// ```ignore
/// let mut output = String::new();
/// escape_string(&mut output, "hello\nworld", QuoteStyle::PreferDouble).unwrap();
/// assert_eq!(output, r#""hello\nworld""#);
/// ```
pub fn escape_string(f: &mut impl fmt::Write, s: &str, style: QuoteStyle) -> fmt::Result {
    // Determine which quote character to use based on style and content
    let (quote_char, needs_escape) = match style {
        QuoteStyle::AlwaysDouble => ('"', '"'),
        QuoteStyle::AlwaysSingle => ('\'', '\''),
        QuoteStyle::PreferDouble => {
            if s.contains('"') && !s.contains('\'') {
                ('\'', '\'')
            } else {
                ('"', '"')
            }
        }
        QuoteStyle::PreferSingle => {
            if s.contains('\'') && !s.contains('"') {
                ('"', '"')
            } else {
                ('\'', '\'')
            }
        }
    };

    write!(f, "{}", quote_char)?;

    for ch in s.chars() {
        if ch == needs_escape {
            write!(f, "\\{}", quote_char)?;
        } else {
            match ch {
                '\\' => write!(f, "\\\\")?,
                '\n' => write!(f, "\\n")?,
                '\r' => write!(f, "\\r")?,
                '\t' => write!(f, "\\t")?,
                '\0' => write!(f, "\\0")?,
                c if c.is_control() => write!(f, "\\u{:04x}", c as u32)?,
                c => write!(f, "{}", c)?,
            }
        }
    }

    write!(f, "{}", quote_char)?;
    Ok(())
}

/// Unescape a Melbi string literal or format string text into its runtime representation.
///
/// This function converts source code string literals into actual strings by processing
/// escape sequences. It supports:
/// - Common escapes: `\n`, `\r`, `\t`, `\\`, `\"`, `\'`, `\0`
/// - Unicode escapes: `\uNNNN` (4 hex digits), `\UNNNNNNNN` (8 hex digits)
/// - Line continuation: `\` followed by newline (removes both, preserves following whitespace)
/// - Format string braces (when `is_format_string=true`): `{{` → `{`, `}}` → `}`
///
/// The function uses arena allocation with an optimization: if the input contains no
/// backslashes (and no format braces for format strings), it returns a direct reference
/// to the input (zero-copy fast path).
///
/// UTF-8 characters are allowed in the source (e.g., `"café"`, `"🌍"`).
///
/// # Arguments
///
/// * `arena` - Arena allocator for the output string (used only if escapes are present)
/// * `input` - The string literal content (without surrounding quotes)
/// * `is_format_string` - If true, also process format string brace escaping (`{{` and `}}`)
///
/// # Returns
///
/// * `Ok(&str)` - The unescaped string (arena-allocated or direct reference to input)
/// * `Err(UnescapeError)` - If an invalid escape sequence or unpaired brace is encountered
///
/// # Example
///
/// ```ignore
/// let arena = Bump::new();
/// let result = unescape_string(&arena, r"hello\nworld", false).unwrap();
/// assert_eq!(result, "hello\nworld");
///
/// let fmt_result = unescape_string(&arena, r"{{literal}}", true).unwrap();
/// assert_eq!(fmt_result, "{literal}");
/// ```
pub fn unescape_string<'a>(
    arena: &'a Bump,
    input: &'a str,
    is_format_string: bool,
) -> Result<&'a str, UnescapeError> {
    // Fast path: no escapes and no format braces, return input directly
    if !input.contains('\\')
        && (!is_format_string
            || (!input.contains("{{")
                && !input.contains("}}")
                && !input.contains('{')
                && !input.contains('}')))
    {
        return Ok(input);
    }

    // Slow path: allocate and process escapes
    // Allocate conservatively (input length is upper bound)
    let output = arena.alloc_slice_fill_default::<u8>(input.len());
    let mut write_pos = 0;
    let mut chars = input.char_indices().peekable();

    while let Some((pos, ch)) = chars.next() {
        // Handle format string brace escaping first
        if is_format_string && (ch == '{' || ch == '}') {
            match chars.peek().map(|(_, c)| *c) {
                Some(next_ch) if next_ch == ch => {
                    // Paired braces: {{ or }}
                    chars.next(); // consume second brace
                    output[write_pos] = ch as u8;
                    write_pos += 1;
                    continue;
                }
                _ => {
                    // Unpaired brace - error
                    return Err(UnescapeError::UnpairedBrace { pos, brace: ch });
                }
            }
        }

        if ch != '\\' {
            // Regular character - encode as UTF-8
            let char_bytes = ch.encode_utf8(&mut output[write_pos..]);
            write_pos += char_bytes.len();
            continue;
        }

        // Process escape sequence
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
            Some((_, '0')) => {
                output[write_pos] = b'\0';
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
            Some((_, '\n')) => {
                // Line continuation: skip both backslash and newline
                // Preserve all following whitespace
                continue;
            }
            Some((upos, 'u')) => {
                // \uNNNN - 4 hex digits
                let hex_start = upos + 1;
                let mut hex_value = 0u32;
                let mut digit_count = 0;

                for _ in 0..4 {
                    match chars.next() {
                        Some((_, ch)) => match ch.to_digit(16) {
                            Some(digit) => {
                                hex_value = (hex_value << 4) | digit;
                                digit_count += 1;
                            }
                            None => {
                                return Err(UnescapeError::InvalidHexDigit {
                                    pos: hex_start,
                                    seq: format!("\\u{}", ch),
                                });
                            }
                        },
                        None => {
                            return Err(UnescapeError::IncompleteUnicodeEscape {
                                pos,
                                expected: 4,
                                got: digit_count,
                            });
                        }
                    }
                }

                // Convert to char
                let unicode_char =
                    char::from_u32(hex_value).ok_or(UnescapeError::InvalidUnicodeScalar {
                        pos,
                        value: hex_value,
                    })?;

                let char_bytes = unicode_char.encode_utf8(&mut output[write_pos..]);
                write_pos += char_bytes.len();
            }
            Some((upos, 'U')) => {
                // \UNNNNNNNN - 8 hex digits
                let hex_start = upos + 1;
                let mut hex_value = 0u32;
                let mut digit_count = 0;

                for _ in 0..8 {
                    match chars.next() {
                        Some((_, ch)) => match ch.to_digit(16) {
                            Some(digit) => {
                                hex_value = (hex_value << 4) | digit;
                                digit_count += 1;
                            }
                            None => {
                                return Err(UnescapeError::InvalidHexDigit {
                                    pos: hex_start,
                                    seq: format!("\\U{}", ch),
                                });
                            }
                        },
                        None => {
                            return Err(UnescapeError::IncompleteUnicodeEscape {
                                pos,
                                expected: 8,
                                got: digit_count,
                            });
                        }
                    }
                }

                // Convert to char
                let unicode_char =
                    char::from_u32(hex_value).ok_or(UnescapeError::InvalidUnicodeScalar {
                        pos,
                        value: hex_value,
                    })?;

                let char_bytes = unicode_char.encode_utf8(&mut output[write_pos..]);
                write_pos += char_bytes.len();
            }
            Some((_, other)) => {
                return Err(UnescapeError::InvalidEscape {
                    pos,
                    seq: format!("\\{}", other),
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

    // Convert the output bytes to a string
    // SAFETY: We only wrote valid UTF-8 to the buffer (either from input chars or encoded chars)
    let result = core::str::from_utf8(&output[..write_pos])
        .expect("BUG: unescape_string produced invalid UTF-8");

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to call escape_string and return a String
    fn escape(s: &str, style: QuoteStyle) -> String {
        let mut output = String::new();
        escape_string(&mut output, s, style).unwrap();
        output
    }

    // ===== escape_string tests =====

    #[test]
    fn test_escape_empty() {
        assert_eq!(escape("", QuoteStyle::PreferDouble), r#""""#);
        assert_eq!(escape("", QuoteStyle::PreferSingle), "''");
    }

    #[test]
    fn test_escape_no_special_chars() {
        assert_eq!(escape("hello", QuoteStyle::PreferDouble), r#""hello""#);
        assert_eq!(escape("hello", QuoteStyle::PreferSingle), "'hello'");
    }

    #[test]
    fn test_escape_common_escapes() {
        assert_eq!(
            escape("hello\nworld", QuoteStyle::PreferDouble),
            r#""hello\nworld""#
        );
        assert_eq!(
            escape("tab\there", QuoteStyle::PreferDouble),
            r#""tab\there""#
        );
        assert_eq!(
            escape("return\rkey", QuoteStyle::PreferDouble),
            r#""return\rkey""#
        );
        assert_eq!(
            escape("back\\slash", QuoteStyle::PreferDouble),
            r#""back\\slash""#
        );
        assert_eq!(
            escape("null\0byte", QuoteStyle::PreferDouble),
            r#""null\0byte""#
        );
    }

    #[test]
    fn test_escape_quotes() {
        assert_eq!(
            escape(r#"say "hi""#, QuoteStyle::AlwaysDouble),
            r#""say \"hi\"""#
        );
        assert_eq!(
            escape("say 'hi'", QuoteStyle::AlwaysSingle),
            r"'say \'hi\''"
        );
    }

    #[test]
    fn test_escape_quote_selection_prefer_double() {
        // No quotes -> double
        assert_eq!(escape("hello", QuoteStyle::PreferDouble), r#""hello""#);
        // Has double but not single -> use single
        assert_eq!(
            escape(r#"say "hi""#, QuoteStyle::PreferDouble),
            r#"'say "hi"'"#
        );
        // Has both -> use double (prefer double)
        assert_eq!(
            escape(r#"say "hi" and 'bye'"#, QuoteStyle::PreferDouble),
            r#""say \"hi\" and 'bye'""#
        );
        // Has single but not double -> use double
        assert_eq!(
            escape("say 'hi'", QuoteStyle::PreferDouble),
            r#""say 'hi'""#
        );
    }

    #[test]
    fn test_escape_quote_selection_prefer_single() {
        // No quotes -> single
        assert_eq!(escape("hello", QuoteStyle::PreferSingle), "'hello'");
        // Has single but not double -> use double
        assert_eq!(
            escape("say 'hi'", QuoteStyle::PreferSingle),
            r#""say 'hi'""#
        );
        // Has both -> use single (prefer single)
        assert_eq!(
            escape(r#"say "hi" and 'bye'"#, QuoteStyle::PreferSingle),
            "'say \"hi\" and \\'bye\\''"
        );
        // Has double but not single -> use single
        assert_eq!(
            escape(r#"say "hi""#, QuoteStyle::PreferSingle),
            r#"'say "hi"'"#
        );
    }

    #[test]
    fn test_escape_control_characters() {
        assert_eq!(
            escape("\x01\x02\x03", QuoteStyle::PreferDouble),
            r#""\u0001\u0002\u0003""#
        );
        assert_eq!(escape("\x7f", QuoteStyle::PreferDouble), r#""\u007f""#);
    }

    #[test]
    fn test_escape_unicode() {
        // UTF-8 characters should pass through as-is (not escaped)
        assert_eq!(escape("café", QuoteStyle::PreferDouble), r#""café""#);
        assert_eq!(escape("🌍", QuoteStyle::PreferDouble), r#""🌍""#);
        assert_eq!(
            escape("hello世界", QuoteStyle::PreferDouble),
            r#""hello世界""#
        );
    }

    // ===== unescape_string tests =====

    #[test]
    fn test_unescape_empty() {
        let arena = Bump::new();
        assert_eq!(unescape_string(&arena, "", false).unwrap(), "");
    }

    #[test]
    fn test_unescape_no_escapes() {
        let arena = Bump::new();
        let input = "hello world";
        // Should return input directly (zero-copy)
        assert_eq!(unescape_string(&arena, input, false).unwrap(), input);
    }

    #[test]
    fn test_unescape_common_escapes() {
        let arena = Bump::new();
        assert_eq!(
            unescape_string(&arena, r"hello\nworld", false).unwrap(),
            "hello\nworld"
        );
        assert_eq!(
            unescape_string(&arena, r"tab\there", false).unwrap(),
            "tab\there"
        );
        assert_eq!(
            unescape_string(&arena, r"return\rkey", false).unwrap(),
            "return\rkey"
        );
        assert_eq!(
            unescape_string(&arena, r"back\\slash", false).unwrap(),
            "back\\slash"
        );
        assert_eq!(
            unescape_string(&arena, r"null\0byte", false).unwrap(),
            "null\0byte"
        );
    }

    #[test]
    fn test_unescape_quotes() {
        let arena = Bump::new();
        assert_eq!(
            unescape_string(&arena, r#"say \"hi\""#, false).unwrap(),
            r#"say "hi""#
        );
        assert_eq!(
            unescape_string(&arena, r"say \'hi\'", false).unwrap(),
            "say 'hi'"
        );
    }

    #[test]
    fn test_unescape_unicode_4digit() {
        let arena = Bump::new();
        assert_eq!(
            unescape_string(&arena, r"\u0048\u0065\u006c\u006c\u006f", false).unwrap(),
            "Hello"
        );
        assert_eq!(
            unescape_string(&arena, r"caf\u00e9", false).unwrap(),
            "café"
        );
        assert_eq!(
            unescape_string(&arena, r"\u4e16\u754c", false).unwrap(),
            "世界"
        );
    }

    #[test]
    fn test_unescape_unicode_8digit() {
        let arena = Bump::new();
        assert_eq!(
            unescape_string(
                &arena,
                r"\U00000048\U00000065\U0000006c\U0000006c\U0000006f",
                false
            )
            .unwrap(),
            "Hello"
        );
        assert_eq!(unescape_string(&arena, r"\U0001F30D", false).unwrap(), "🌍");
        assert_eq!(unescape_string(&arena, r"\U0001F44B", false).unwrap(), "👋");
    }

    #[test]
    fn test_unescape_mixed_unicode() {
        let arena = Bump::new();
        assert_eq!(
            unescape_string(&arena, r"Hello \u4e16\u754c \U0001F30D", false).unwrap(),
            "Hello 世界 🌍"
        );
    }

    #[test]
    fn test_unescape_utf8_in_source() {
        let arena = Bump::new();
        // UTF-8 characters in source should pass through
        assert_eq!(unescape_string(&arena, "café", false).unwrap(), "café");
        assert_eq!(unescape_string(&arena, "🌍", false).unwrap(), "🌍");
        assert_eq!(
            unescape_string(&arena, "hello世界", false).unwrap(),
            "hello世界"
        );
    }

    #[test]
    fn test_unescape_line_continuation() {
        let arena = Bump::new();
        // Backslash + newline should be removed
        assert_eq!(
            unescape_string(&arena, "hello\\\nworld", false).unwrap(),
            "helloworld"
        );
        // Following whitespace should be preserved
        assert_eq!(
            unescape_string(&arena, "hello\\\n  world", false).unwrap(),
            "hello  world"
        );
        assert_eq!(
            unescape_string(&arena, "one\\\n\ttwo", false).unwrap(),
            "one\ttwo"
        );
    }

    #[test]
    fn test_unescape_invalid_escape() {
        let arena = Bump::new();
        assert!(matches!(
            unescape_string(&arena, r"\q", false),
            Err(UnescapeError::InvalidEscape { pos: 0, .. })
        ));
        assert!(matches!(
            unescape_string(&arena, r"hello\x42", false),
            Err(UnescapeError::InvalidEscape { pos: 5, .. })
        ));
    }

    #[test]
    fn test_unescape_invalid_hex_digit() {
        let arena = Bump::new();
        assert!(matches!(
            unescape_string(&arena, r"\u00GG", false),
            Err(UnescapeError::InvalidHexDigit { .. })
        ));
        assert!(matches!(
            unescape_string(&arena, r"\U0000000G", false),
            Err(UnescapeError::InvalidHexDigit { .. })
        ));
    }

    #[test]
    fn test_unescape_incomplete_unicode() {
        let arena = Bump::new();
        assert!(matches!(
            unescape_string(&arena, r"\u00", false),
            Err(UnescapeError::IncompleteUnicodeEscape {
                expected: 4,
                got: 2,
                ..
            })
        ));
        assert!(matches!(
            unescape_string(&arena, r"\U000000", false),
            Err(UnescapeError::IncompleteUnicodeEscape {
                expected: 8,
                got: 6,
                ..
            })
        ));
    }

    #[test]
    fn test_unescape_invalid_unicode_scalar() {
        let arena = Bump::new();
        // D800-DFFF are surrogate code points (invalid in UTF-8)
        assert!(matches!(
            unescape_string(&arena, r"\uD800", false),
            Err(UnescapeError::InvalidUnicodeScalar { value: 0xD800, .. })
        ));
        // Values above 0x10FFFF are invalid
        assert!(matches!(
            unescape_string(&arena, r"\U00110000", false),
            Err(UnescapeError::InvalidUnicodeScalar {
                value: 0x110000,
                ..
            })
        ));
    }

    #[test]
    fn test_unescape_incomplete_at_end() {
        let arena = Bump::new();
        assert!(matches!(
            unescape_string(&arena, r"hello\", false),
            Err(UnescapeError::InvalidEscape { pos: 5, .. })
        ));
    }

    #[test]
    fn test_roundtrip() {
        let arena = Bump::new();
        let test_cases = [
            "hello world",
            "hello\nworld",
            "tab\there",
            r#"say "hi""#,
            "say 'hi'",
            "café",
            "🌍",
            "hello世界",
            "null\0byte",
        ];

        for &test in &test_cases {
            let escaped = escape(test, QuoteStyle::PreferDouble);
            // Remove surrounding quotes
            let escaped_inner = &escaped[1..escaped.len() - 1];
            let unescaped = unescape_string(&arena, escaped_inner, false).unwrap();
            assert_eq!(unescaped, test, "Roundtrip failed for: {:?}", test);
        }
    }

    // ===== Format string mode tests =====

    #[test]
    fn test_unescape_format_braces() {
        let arena = Bump::new();
        // Double braces should become single braces
        assert_eq!(unescape_string(&arena, "{{", true).unwrap(), "{");
        assert_eq!(unescape_string(&arena, "}}", true).unwrap(), "}");
        assert_eq!(
            unescape_string(&arena, "{{hello}}", true).unwrap(),
            "{hello}"
        );
        assert_eq!(unescape_string(&arena, "a{{b}}c", true).unwrap(), "a{b}c");
    }

    #[test]
    fn test_unescape_format_with_escapes() {
        let arena = Bump::new();
        // Combine brace escaping with string escapes
        assert_eq!(unescape_string(&arena, r"{{\n}}", true).unwrap(), "{\n}");
        assert_eq!(
            unescape_string(&arena, r"Line 1\nLine 2\t{{literal}}", true).unwrap(),
            "Line 1\nLine 2\t{literal}"
        );
        assert_eq!(
            unescape_string(&arena, r"{{prefix}}\n{{suffix}}", true).unwrap(),
            "{prefix}\n{suffix}"
        );
    }

    #[test]
    fn test_unescape_format_complex() {
        let arena = Bump::new();
        // Test combinations
        assert_eq!(
            unescape_string(&arena, r"hello {{\n}} world", true).unwrap(),
            "hello {\n} world"
        );
        assert_eq!(
            unescape_string(&arena, r"{{a}}\t{{b}}", true).unwrap(),
            "{a}\t{b}"
        );
        assert_eq!(
            unescape_string(&arena, r"Test: {{\u0048i}}", true).unwrap(),
            "Test: {Hi}"
        );
    }

    #[test]
    fn test_unescape_format_unpaired_brace_left() {
        let arena = Bump::new();
        // NOTE: The grammar (format_text rule) prevents unpaired braces from reaching
        // this function in normal parsing. This test verifies the defensive behavior
        // if the function is called directly with invalid input.
        // Single { should be an error
        assert!(matches!(
            unescape_string(&arena, "hello {", true),
            Err(UnescapeError::UnpairedBrace { pos: 6, brace: '{' })
        ));
        assert!(matches!(
            unescape_string(&arena, "{ world", true),
            Err(UnescapeError::UnpairedBrace { pos: 0, brace: '{' })
        ));
    }

    #[test]
    fn test_unescape_format_unpaired_brace_right() {
        let arena = Bump::new();
        // NOTE: The grammar (format_text rule) prevents unpaired braces from reaching
        // this function in normal parsing. This test verifies the defensive behavior
        // if the function is called directly with invalid input.
        // Single } should be an error
        assert!(matches!(
            unescape_string(&arena, "hello }", true),
            Err(UnescapeError::UnpairedBrace { pos: 6, brace: '}' })
        ));
        assert!(matches!(
            unescape_string(&arena, "} world", true),
            Err(UnescapeError::UnpairedBrace { pos: 0, brace: '}' })
        ));
    }

    #[test]
    fn test_unescape_format_no_braces_same_as_normal() {
        let arena = Bump::new();
        // If there are no braces, format mode should behave like normal mode
        let test_cases = ["hello", r"hello\nworld", r"\u0048i"];
        for test in test_cases {
            let normal = unescape_string(&arena, test, false).unwrap();
            let format = unescape_string(&arena, test, true).unwrap();
            assert_eq!(normal, format, "Mismatch for: {}", test);
        }
    }
}
