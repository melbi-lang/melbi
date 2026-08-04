use pest::Parser;
use pest::error::Error;

use super::parser::{ExpressionParser, Rule};

#[test]
fn valid_expressions() -> Result<(), Error<Rule>> {
    let examples = [
        "1 + 2",
        "a * b + c",
        "if x then y else z",
        "x where {x = 1}",
        "foo(1, 2, 3)",
        "[1, 2, 3]",
        "{a: 1, b: 2}",
        "{x = 42}",
        "Record {}",
        "`0` + `some-name`",
        "1 as Integer",
        "\"abc\" as Bytes",
        "{ x = 1 } as Record[x: Integer]",
        "f\"Hello, {name}!\"",
        "'\\n'", // string escape
        "'\\u0041'",
        "b\"\\x41\"",
        "Record", // valid as identifier
        // Integer literals with different bases
        "42",
        "0b1010",
        "0o755",
        "0xDEADBEEF",
        "1_000_000",
        "0b1010_1010",
        "0o7_5_5",
        "0xDEAD_BEEF",
        "0b_1010", // underscore immediately after prefix
        "42_",     // trailing underscore
        // Integer literals with suffixes (quoted only)
        "42`kg`",
        "100`USD`",
        "255`u8`",
        "42`m/s`",
        // Float literals
        "3.14",
        "3.",
        ".5",
        "1.0e10",
        "6.022e23",
        "1e-10",
        ".5e2",
        "3.14e-5",
        "1_000.5_000",
        "1_000_000e10",
        "1.5_", // trailing underscore in fractional part
        // Float literals with suffixes (quoted only)
        "3.14`f32`",
        "6.022e23`mol`",
        "1.0`kg`",
        "3.14`rad/s`",
        // Multiple consecutive underscores are allowed
        "1__000__000",
    ];

    for expr in examples {
        ExpressionParser::parse(Rule::main, expr)
            .unwrap_or_else(|e| panic!("Failed to parse '{}': {}", expr, e));
    }

    Ok(())
}

#[test]
fn invalid_expressions() {
    let examples = [
        "1 +",
        "if x then else y",
        "{a: }",
        "Record[]",     // type, not value
        "1 as",         // missing type
        "1 as \"Int\"", // invalid type expression
        "`",            // unterminated quoted ident
        "b\"\\u0041\"", // invalid unicode escape in bytes
        "\"\\x41\"",    // invalid hex escape in string
        // Invalid suffixes (unquoted suffixes no longer allowed)
        "42kg",    // unquoted suffix
        "3.14f32", // unquoted suffix
        "100USD",  // unquoted suffix
        "3eV",     // looks like exponent but invalid
        // Edge cases: base prefixes with invalid/missing digits fail to parse
        "0b2", // would parse as 0 with unquoted suffix "b2" - not allowed
        "0o8", // would parse as 0 with unquoted suffix "o8" - not allowed
        "0xG", // would parse as 0 with unquoted suffix "xG" - not allowed
        "0b",  // would parse as 0 with unquoted suffix "b" - not allowed
        "0x",  // would parse as 0 with unquoted suffix "x" - not allowed
        // Invalid float literals
        "1.2.3", // multiple decimal points
        "1e",    // exponent without number
        "1.e",   // exponent without number (now fails correctly)
        ".e10",  // missing digits before exponent
        "1e+",   // exponent sign without number
        "._5",   // underscore immediately after decimal
        "1._5",  // underscore immediately after decimal (now fails correctly)
    ];

    for expr in examples {
        assert!(
            ExpressionParser::parse(Rule::main, expr).is_err(),
            "Expected failure parsing '{}'",
            expr
        );
    }
}

#[test]
fn pattern_matching_syntax() {
    use bumpalo::Bump;

    use crate::parser;

    let examples = [
        // Basic patterns
        "x match { 1 -> 2, _ -> 3 }",
        "x match { true -> 1, false -> 0 }",
        "x match { _ -> 42 }",
        // Variable patterns
        "x match { y -> y }",
        "value match { x -> x + 1 }",
        // Option patterns
        "option match { some x -> x, none -> 0 }",
        "x match { some y -> y, none -> -1 }",
        // Nested option patterns
        "x match { some (some y) -> y, some none -> -1, none -> 0 }",
        "value match { some (some (some z)) -> z, _ -> 0 }",
        // Literal patterns
        "x match { 1 -> 'one', 2 -> 'two', _ -> 'other' }",
        "x match { 3.14 -> 'pi', 2.71 -> 'e', _ -> 'unknown' }",
        "x match { \"hello\" -> 1, \"world\" -> 2, _ -> 0 }",
        "x match { b\"abc\" -> true, _ -> false }",
        // Mixed patterns
        "x match { none -> 0, some 1 -> 10, some _ -> 100 }",
        // With trailing comma
        "x match { 1 -> 2, }",
        "x match { some x -> x, none -> 0, }",
    ];

    let arena = Bump::new();
    for expr in examples {
        parser::parse(&arena, expr).unwrap_or_else(|e| panic!("Failed to parse '{}': {}", expr, e));
    }
}
