//! Tests for the String package

use super::build_string_package;
use crate::{
    api::{CompileOptionsOverride, Engine, EngineOptions},
    types::manager::TypeManager, // This import is necessary for test helpers
    values::{builder::Binder, dynamic::Value},
};
use bumpalo::Bump;

#[test]
fn test_string_package_builds() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let string = build_string_package(&arena, type_mgr).unwrap();
    let record = string.as_record().unwrap();

    // Should have all functions
    assert!(!record.is_empty());
}

// Helper function for integration tests using the Engine to evaluate Melbi code
fn test_string_expr<F>(source: &str, check: F)
where
    F: FnOnce(Value),
{
    let options = EngineOptions::default();
    let arena = Bump::new();

    let engine = Engine::new(options, &arena, |arena, type_mgr, env| {
        let string = build_string_package(arena, type_mgr).unwrap();
        env.bind("String", string)
    });

    let compile_opts = CompileOptionsOverride::default();
    let expr = engine
        .compile(compile_opts, source, &[])
        .expect("compilation should succeed");

    let val_arena = Bump::new();
    let result = expr
        .run(Default::default(), &val_arena, &[])
        .expect("execution should succeed");

    check(result);
}

#[test]
fn test_string_len() {
    // ASCII string
    test_string_expr("String.Len(\"hello\")", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 5);
    });

    // UTF-8 string (codepoints, not bytes)
    test_string_expr("String.Len(\"café\")", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 4); // Not 5 bytes
    });

    // Empty string
    test_string_expr("String.Len(\"\")", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });

    // Emoji (single codepoint)
    test_string_expr("String.Len(\"👍\")", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 1);
    });
}

#[test]
fn test_string_is_empty() {
    test_string_expr("String.IsEmpty(\"\")", |r: Value| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    test_string_expr("String.IsEmpty(\"hello\")", |r: Value| {
        assert_eq!(r.as_bool().unwrap(), false);
    });
}

#[test]
fn test_string_contains() {
    test_string_expr("String.Contains(\"hello world\", \"world\")", |r: Value| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    test_string_expr("String.Contains(\"hello\", \"goodbye\")", |r: Value| {
        assert_eq!(r.as_bool().unwrap(), false);
    });
}

#[test]
fn test_string_starts_with() {
    test_string_expr("String.StartsWith(\"hello\", \"hel\")", |r: Value| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    test_string_expr("String.StartsWith(\"hello\", \"llo\")", |r: Value| {
        assert_eq!(r.as_bool().unwrap(), false);
    });
}

#[test]
fn test_string_ends_with() {
    test_string_expr("String.EndsWith(\"hello\", \"llo\")", |r: Value| {
        assert_eq!(r.as_bool().unwrap(), true);
    });

    test_string_expr("String.EndsWith(\"hello\", \"hel\")", |r: Value| {
        assert_eq!(r.as_bool().unwrap(), false);
    });
}

#[test]
fn test_string_upper() {
    // ASCII only
    test_string_expr("String.Upper(\"hello\")", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "HELLO");
    });

    test_string_expr("String.Upper(\"Hello123\")", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "HELLO123");
    });

    // Unicode characters pass through unchanged (ASCII-only operation)
    test_string_expr("String.Upper(\"café\")", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "CAFé"); // 'é' unchanged
    });
}

#[test]
fn test_string_lower() {
    // ASCII only
    test_string_expr("String.Lower(\"HELLO\")", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "hello");
    });

    test_string_expr("String.Lower(\"Hello123\")", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "hello123");
    });

    // Unicode characters pass through unchanged (ASCII-only operation)
    test_string_expr("String.Lower(\"CAFÉ\")", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "cafÉ"); // 'É' unchanged
    });
}

#[test]
fn test_string_trim() {
    test_string_expr("String.Trim(\"  hello  \")", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "hello");
    });

    test_string_expr("String.Trim(\"hello\")", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "hello");
    });

    test_string_expr("String.Trim(\"  \")", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "");
    });
}

#[test]
fn test_string_trim_start() {
    test_string_expr("String.TrimStart(\"  hello  \")", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "hello  ");
    });
}

#[test]
fn test_string_trim_end() {
    test_string_expr("String.TrimEnd(\"  hello  \")", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "  hello");
    });
}

#[test]
fn test_string_replace() {
    test_string_expr(
        "String.Replace(\"hello world\", \"world\", \"Melbi\")",
        |r: Value| {
            assert_eq!(r.as_str().unwrap(), "hello Melbi");
        },
    );

    test_string_expr("String.Replace(\"aaa\", \"a\", \"b\")", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "bbb"); // Replaces all
    });
}

#[test]
fn test_string_replace_n() {
    test_string_expr("String.ReplaceN(\"aaa\", \"a\", \"b\", 2)", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "bba"); // Only first 2
    });

    test_string_expr("String.ReplaceN(\"hello\", \"l\", \"L\", 1)", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "heLlo"); // Only first occurrence
    });
}

#[test]
fn test_string_split() {
    test_string_expr("String.Split(\"a,b,c\", \",\")", |r: Value| {
        let arr = r.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr.get(0).unwrap().as_str().unwrap(), "a");
        assert_eq!(arr.get(1).unwrap().as_str().unwrap(), "b");
        assert_eq!(arr.get(2).unwrap().as_str().unwrap(), "c");
    });

    test_string_expr("String.Split(\"hello\", \"\")", |r: Value| {
        let arr = r.as_array().unwrap();
        assert_eq!(arr.len(), 5); // Splits into chars
    });
}

#[test]
fn test_string_join() {
    test_string_expr("String.Join([\"a\", \"b\", \"c\"], \",\")", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "a,b,c");
    });

    test_string_expr("String.Join([\"hello\"], \",\")", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "hello");
    });
}

#[test]
fn test_string_substring() {
    // Normal substring
    test_string_expr("String.Substring(\"hello\", 1, 4)", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "ell");
    });

    // Full string
    test_string_expr("String.Substring(\"hello\", 0, 5)", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "hello");
    });

    // Out of bounds (clamped)
    test_string_expr("String.Substring(\"hello\", 0, 100)", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "hello");
    });

    // Empty substring
    test_string_expr("String.Substring(\"hello\", 2, 2)", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "");
    });

    // UTF-8 substring (by codepoints)
    test_string_expr("String.Substring(\"café\", 0, 3)", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "caf");
    });
}

#[test]
fn test_string_to_int() {
    // Valid integer
    test_string_expr("String.ToInt(\"42\")", |r: Value| {
        let opt = r.as_option().unwrap();
        assert!(opt.is_some());
        assert_eq!(opt.unwrap().as_int().unwrap(), 42);
    });

    // Negative integer
    test_string_expr("String.ToInt(\"-123\")", |r: Value| {
        let opt = r.as_option().unwrap();
        assert!(opt.is_some());
        assert_eq!(opt.unwrap().as_int().unwrap(), -123);
    });

    // Invalid integer
    test_string_expr("String.ToInt(\"abc\")", |r: Value| {
        let opt = r.as_option().unwrap();
        assert!(opt.is_none());
    });

    // Float string (invalid for ToInt)
    test_string_expr("String.ToInt(\"3.14\")", |r: Value| {
        let opt = r.as_option().unwrap();
        assert!(opt.is_none());
    });
}

#[test]
fn test_string_to_float() {
    // Valid float
    test_string_expr("String.ToFloat(\"3.14\")", |r: Value| {
        let opt = r.as_option().unwrap();
        assert!(opt.is_some());
        assert_eq!(opt.unwrap().as_float().unwrap(), 3.14);
    });

    // Integer string (valid for ToFloat)
    test_string_expr("String.ToFloat(\"42\")", |r: Value| {
        let opt = r.as_option().unwrap();
        assert!(opt.is_some());
        assert_eq!(opt.unwrap().as_float().unwrap(), 42.0);
    });

    // Invalid float
    test_string_expr("String.ToFloat(\"abc\")", |r: Value| {
        let opt = r.as_option().unwrap();
        assert!(opt.is_none());
    });

    // Scientific notation
    test_string_expr("String.ToFloat(\"1e10\")", |r: Value| {
        let opt = r.as_option().unwrap();
        assert!(opt.is_some());
        assert_eq!(opt.unwrap().as_float().unwrap(), 1e10);
    });
}

#[test]
fn test_string_composition() {
    // Combining multiple string operations
    test_string_expr("String.Upper(String.Trim(\"  hello  \"))", |r: Value| {
        assert_eq!(r.as_str().unwrap(), "HELLO");
    });

    test_string_expr(
        "String.Len(String.Replace(\"aaa\", \"a\", \"bb\"))",
        |r: Value| {
            assert_eq!(r.as_int().unwrap(), 6); // "bbbbbb"
        },
    );
}
