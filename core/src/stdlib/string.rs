//! String Package
//!
//! Provides string manipulation functions for Melbi.
//!
//! Design notes:
//! - String.Len returns UTF-8 codepoint count (not byte count)
//! - Upper/Lower are ASCII-only to keep binary size minimal
//! - For full Unicode support, use the Unicode package
//! - Format strings (f"...") are built into the language, not library functions

use crate::{
    Vec,
    values::{
        FfiContext,
        typed::{Array, Optional, Str},
    },
};
use melbi_macros::{melbi_fn, melbi_package};

#[melbi_package]
mod string {
    use super::*;

    // ========================================================================
    // Inspection Functions
    // ========================================================================

    /// Get the length of a string (number of UTF-8 codepoints, not bytes)
    #[melbi_fn]
    fn len(s: Str) -> i64 {
        s.chars().count() as i64
    }

    /// Check if string is empty
    #[melbi_fn]
    fn is_empty(s: Str) -> bool {
        s.is_empty()
    }

    /// Check if haystack contains needle
    #[melbi_fn]
    fn contains(haystack: Str, needle: Str) -> bool {
        haystack.contains(needle.as_ref())
    }

    /// Check if string starts with prefix
    #[melbi_fn]
    fn starts_with(s: Str, prefix: Str) -> bool {
        s.starts_with(prefix.as_ref())
    }

    /// Check if string ends with suffix
    #[melbi_fn]
    fn ends_with(s: Str, suffix: Str) -> bool {
        s.ends_with(suffix.as_ref())
    }

    // ========================================================================
    // Transformation Functions (ASCII-only for minimal binary size)
    // ========================================================================

    /// Convert string to uppercase (ASCII-only)
    #[melbi_fn]
    fn upper<'a>(ctx: &FfiContext<'_, 'a>, s: Str<'a>) -> Str<'a> {
        let upper = s.to_ascii_uppercase();
        Str::from_str(ctx.arena(), &upper)
    }

    /// Convert string to lowercase (ASCII-only)
    #[melbi_fn]
    fn lower<'a>(ctx: &FfiContext<'_, 'a>, s: Str<'a>) -> Str<'a> {
        let lower = s.to_ascii_lowercase();
        Str::from_str(ctx.arena(), &lower)
    }

    /// Trim whitespace from both ends
    #[melbi_fn]
    fn trim<'a>(ctx: &FfiContext<'_, 'a>, s: Str<'a>) -> Str<'a> {
        let trimmed = s.as_str().trim();
        Str::from_borrowed_str(ctx.arena(), trimmed)
    }

    /// Trim whitespace from start
    #[melbi_fn]
    fn trim_start<'a>(ctx: &FfiContext<'_, 'a>, s: Str<'a>) -> Str<'a> {
        let trimmed = s.as_str().trim_start();
        Str::from_borrowed_str(ctx.arena(), trimmed)
    }

    /// Trim whitespace from end
    #[melbi_fn]
    fn trim_end<'a>(ctx: &FfiContext<'_, 'a>, s: Str<'a>) -> Str<'a> {
        let trimmed = s.as_str().trim_end();
        Str::from_borrowed_str(ctx.arena(), trimmed)
    }

    /// Replace all occurrences of pattern with replacement
    #[melbi_fn]
    fn replace<'a>(ctx: &FfiContext<'_, 'a>, s: Str<'a>, from: Str<'a>, to: Str<'a>) -> Str<'a> {
        let replaced = s.replace(from.as_ref(), to.as_ref());
        Str::from_str(ctx.arena(), &replaced)
    }

    /// Replace first N occurrences of pattern with replacement
    #[melbi_fn]
    fn replace_n<'a>(
        ctx: &FfiContext<'_, 'a>,
        s: Str<'a>,
        from: Str<'a>,
        to: Str<'a>,
        count: i64,
    ) -> Str<'a> {
        let replaced = s.replacen(from.as_ref(), to.as_ref(), count as usize);
        Str::from_str(ctx.arena(), &replaced)
    }

    // ========================================================================
    // Splitting and Joining
    // ========================================================================

    /// Split string by delimiter
    ///
    /// Special case: empty delimiter splits into individual characters (codepoints)
    #[melbi_fn]
    fn split<'a>(ctx: &FfiContext<'_, 'a>, s: Str<'a>, delimiter: Str<'a>) -> Array<'a, Str<'a>> {
        let parts: Vec<Str<'a>> = if delimiter.is_empty() {
            // Empty delimiter: split into individual characters (codepoints)
            // Note: This case still requires allocation since we need to create individual char strings
            s.as_str()
                .chars()
                .map(|c| {
                    let char_str = alloc::string::String::from(c);
                    Str::from_str(ctx.arena(), &char_str)
                })
                .collect()
        } else {
            // Non-empty delimiter: use standard split (zero-copy substrings)
            s.as_str()
                .split(delimiter.as_ref())
                .map(|part| Str::from_borrowed_str(ctx.arena(), part))
                .collect()
        };

        Array::new(ctx.arena(), &parts)
    }

    /// Join array of strings with separator
    #[melbi_fn]
    fn join<'a>(
        ctx: &FfiContext<'_, 'a>,
        parts: Array<'a, Str<'a>>,
        separator: Str<'a>,
    ) -> Str<'a> {
        let strings: Vec<&'a str> = parts.iter().map(|s: Str<'a>| s.as_str()).collect();
        let joined = strings.join(separator.as_ref());
        Str::from_str(ctx.arena(), &joined)
    }

    // ========================================================================
    // Extraction
    // ========================================================================

    /// Extract substring by codepoint indices (not byte indices)
    ///
    /// Returns a substring from `start` (inclusive) to `end` (exclusive) by UTF-8 codepoint positions.
    ///
    /// # Edge Cases
    ///
    /// - If `start >= end`, returns an empty string
    /// - If `start` is beyond the string length, returns an empty string
    /// - If `end` is beyond the string length, it's clamped to the string length
    /// - Indices are in codepoints (Unicode scalar values), not bytes
    ///
    /// # Performance
    ///
    /// This operation is O(n) where n is the string length, as it must count UTF-8 codepoints
    /// to find byte positions. The resulting substring is zero-copy (shares the original string's data).
    #[melbi_fn]
    fn substring<'a>(ctx: &FfiContext<'_, 'a>, s: Str<'a>, start: i64, end: i64) -> Str<'a> {
        let start_idx = start as usize;
        let end_idx = end as usize;

        let s_str = s.as_str();

        // Find byte positions for the codepoint indices using char_indices
        let mut byte_start = None;
        let mut byte_end = s_str.len(); // default to end of string

        for (char_pos, (byte_pos, _)) in s_str.char_indices().enumerate() {
            if char_pos == start_idx {
                byte_start = Some(byte_pos);
            }
            if char_pos == end_idx {
                byte_end = byte_pos;
                break;
            }
        }

        // If start is beyond the string, return empty
        let byte_start = match byte_start {
            Some(pos) => pos,
            None => return Str::from_str(ctx.arena(), ""),
        };

        // If start >= end, return empty
        if byte_start >= byte_end {
            return Str::from_str(ctx.arena(), "");
        }

        // Zero-copy substring
        let substring = &s_str[byte_start..byte_end];
        Str::from_borrowed_str(ctx.arena(), substring)
    }

    // ========================================================================
    // Parsing
    // ========================================================================

    /// Parse string to integer
    #[melbi_fn]
    fn to_int<'a>(ctx: &FfiContext<'_, 'a>, s: Str<'a>) -> Optional<'a, i64> {
        match s.parse::<i64>() {
            Ok(value) => Optional::some(ctx.arena(), value),
            Err(_) => Optional::none(),
        }
    }

    /// Parse string to float
    #[melbi_fn]
    fn to_float<'a>(ctx: &FfiContext<'_, 'a>, s: Str<'a>) -> Optional<'a, f64> {
        match s.parse::<f64>() {
            Ok(value) => Optional::some(ctx.arena(), value),
            Err(_) => Optional::none(),
        }
    }
}

// Re-export everything for cleaner access
pub use self::string::*;

#[cfg(test)]
#[path = "string_test.rs"]
mod string_test;
