//! Tests for both unescapers.
//!
//! Failure cases assert the error *kind* and its position, not just that
//! something failed: the position is what a diagnostic underlines, and it is the
//! part most easily got wrong.

use alloc::vec::Vec;

use super::bytes::{UnescapeError as BytesError, unescape_bytes};
use super::string::{UnescapeError as StringError, unescape_string};

// --- helpers -----------------------------------------------------------------

#[track_caller]
fn unescape(input: &str) -> alloc::string::String {
    unescape_string(input, false).expect("expected the literal to unescape")
}

#[track_caller]
fn unescape_format(input: &str) -> alloc::string::String {
    unescape_string(input, true).expect("expected the format literal to unescape")
}

#[track_caller]
fn string_error(input: &str, is_format_string: bool) -> StringError {
    unescape_string(input, is_format_string).expect_err("expected the literal to be rejected")
}

#[track_caller]
fn unescape_b(input: &str) -> Vec<u8> {
    unescape_bytes(input).expect("expected the bytes literal to unescape")
}

#[track_caller]
fn bytes_error(input: &str) -> BytesError {
    unescape_bytes(input).expect_err("expected the bytes literal to be rejected")
}

// --- strings: success --------------------------------------------------------

#[test]
fn plain_string_passes_through() {
    assert_eq!(unescape(""), "");
    assert_eq!(unescape("hello"), "hello");
    // Non-ASCII is fine in a string, unlike in a bytes literal.
    assert_eq!(unescape("héllo → 🌍"), "héllo → 🌍");
}

#[test]
fn simple_escapes_resolve() {
    assert_eq!(unescape(r"a\nb"), "a\nb");
    assert_eq!(unescape(r"a\rb"), "a\rb");
    assert_eq!(unescape(r"a\tb"), "a\tb");
    assert_eq!(unescape(r"a\0b"), "a\0b");
    assert_eq!(unescape(r"a\\b"), r"a\b");
    assert_eq!(unescape(r#"a\"b"#), "a\"b");
    assert_eq!(unescape(r"a\'b"), "a'b");
}

#[test]
fn unicode_escapes_resolve() {
    assert_eq!(unescape("\\u0041"), "A");
    assert_eq!(unescape(r"\U00000041"), "A");
    // Above the BMP, so it does not fit in the 4-digit form.
    assert_eq!(unescape(r"\U0001F30D"), "🌍");
    // Case-insensitive digits, and adjacent text is untouched.
    assert_eq!(unescape("[\\u00e9\\u00E9]"), "[éé]");
}

#[test]
fn line_continuation_drops_the_newline_and_keeps_indentation() {
    assert_eq!(unescape("a\\\n    b"), "a    b");
}

#[test]
fn format_string_collapses_doubled_braces() {
    assert_eq!(unescape_format("{{"), "{");
    assert_eq!(unescape_format("}}"), "}");
    assert_eq!(unescape_format("a{{b}}c"), "a{b}c");
    // Escapes still work alongside braces.
    assert_eq!(unescape_format(r"{{\n}}"), "{\n}");
}

#[test]
fn braces_are_ordinary_outside_a_format_string() {
    assert_eq!(unescape("{"), "{");
    assert_eq!(unescape("a{b}c"), "a{b}c");
}

// --- strings: failure --------------------------------------------------------

#[test]
fn unknown_escape_is_rejected() {
    assert_eq!(
        string_error(r"a\qb", false),
        StringError::InvalidEscape {
            pos: 1,
            seq: r"\q".into()
        }
    );
}

#[test]
fn trailing_backslash_is_rejected() {
    assert_eq!(
        string_error(r"ab\", false),
        StringError::InvalidEscape {
            pos: 2,
            seq: r"\".into()
        }
    );
}

#[test]
fn non_hex_digit_in_unicode_escape_is_rejected() {
    assert_eq!(
        string_error(r"\u00g1", false),
        StringError::InvalidHexDigit {
            pos: 2,
            seq: r"\ug".into()
        }
    );
}

#[test]
fn truncated_unicode_escape_is_rejected() {
    assert_eq!(
        string_error(r"\u12", false),
        StringError::IncompleteUnicodeEscape {
            pos: 0,
            expected: 4,
            got: 2,
        }
    );
    assert_eq!(
        string_error(r"\U0001F3", false),
        StringError::IncompleteUnicodeEscape {
            pos: 0,
            expected: 8,
            got: 6,
        }
    );
}

#[test]
fn surrogate_and_out_of_range_scalars_are_rejected() {
    // A lone surrogate is not a `char`.
    assert_eq!(
        string_error(r"\uD800", false),
        StringError::InvalidUnicodeScalar {
            pos: 0,
            value: 0xD800
        }
    );
    // Beyond the highest code point.
    assert_eq!(
        string_error(r"\UFFFFFFFF", false),
        StringError::InvalidUnicodeScalar {
            pos: 0,
            value: 0xFFFF_FFFF
        }
    );
}

#[test]
fn unpaired_brace_in_a_format_string_is_rejected() {
    assert_eq!(
        string_error("a{b", true),
        StringError::UnpairedBrace {
            pos: 1,
            brace: '{'
        }
    );
    assert_eq!(
        string_error("a}b", true),
        StringError::UnpairedBrace {
            pos: 1,
            brace: '}'
        }
    );
    // A brace at the very end has nothing to pair with.
    assert_eq!(
        string_error("a{", true),
        StringError::UnpairedBrace {
            pos: 1,
            brace: '{'
        }
    );
}

// --- bytes: success ----------------------------------------------------------

#[test]
fn plain_bytes_pass_through() {
    assert_eq!(unescape_b(""), b"");
    assert_eq!(unescape_b("hello"), b"hello");
}

#[test]
fn bytes_simple_escapes_resolve() {
    assert_eq!(unescape_b(r"a\nb"), b"a\nb");
    assert_eq!(unescape_b(r"a\rb"), b"a\rb");
    assert_eq!(unescape_b(r"a\tb"), b"a\tb");
    assert_eq!(unescape_b(r"a\0b"), b"a\0b");
    assert_eq!(unescape_b(r"a\\b"), br"a\b");
    assert_eq!(unescape_b(r#"a\"b"#), b"a\"b");
    assert_eq!(unescape_b(r"a\'b"), b"a'b");
}

#[test]
fn hex_escapes_resolve() {
    assert_eq!(unescape_b(r"\x48\x65\x6c\x6c\x6f"), b"Hello");
    // Case-insensitive, and reaches bytes no character could write directly.
    assert_eq!(unescape_b(r"\xFF\xff\x00"), &[0xFF, 0xFF, 0x00]);
}

#[test]
fn bytes_line_continuation_accepts_lf_and_crlf() {
    assert_eq!(unescape_b("a\\\nb"), b"ab");
    assert_eq!(unescape_b("a\\\r\nb"), b"ab");
    assert_eq!(unescape_b("a\\\rb"), b"ab");
}

// --- bytes: failure ----------------------------------------------------------

#[test]
fn non_ascii_in_bytes_is_rejected() {
    assert_eq!(
        bytes_error("aé"),
        BytesError::NonAsciiCharacter {
            pos: 1,
            character: 'é'
        }
    );
}

#[test]
fn bytes_unknown_escape_is_rejected() {
    assert_eq!(
        bytes_error(r"a\zb"),
        BytesError::InvalidEscape {
            pos: 1,
            seq: r"\z".into()
        }
    );
    // `\u` is a string escape, and deliberately not a bytes one.
    assert_eq!(
        bytes_error("\\u0041"),
        BytesError::InvalidEscape {
            pos: 0,
            seq: "\\u".into()
        }
    );
}

#[test]
fn bytes_trailing_backslash_is_rejected() {
    assert_eq!(
        bytes_error(r"ab\"),
        BytesError::InvalidEscape {
            pos: 2,
            seq: r"\".into()
        }
    );
}

#[test]
fn truncated_hex_escape_is_rejected() {
    assert_eq!(bytes_error(r"\x"), BytesError::IncompleteHexEscape { pos: 0 });
    assert_eq!(
        bytes_error(r"a\x0"),
        BytesError::IncompleteHexEscape { pos: 1 }
    );
}

#[test]
fn non_hex_digit_in_hex_escape_is_rejected() {
    assert_eq!(
        bytes_error(r"\xZZ"),
        BytesError::InvalidHexDigit {
            pos: 0,
            seq: r"\xZZ".into()
        }
    );
    assert_eq!(
        bytes_error(r"\x0g"),
        BytesError::InvalidHexDigit {
            pos: 0,
            seq: r"\x0g".into()
        }
    );
}
