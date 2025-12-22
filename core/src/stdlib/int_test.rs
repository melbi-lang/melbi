//! Tests for the Int package

use super::build_int_package;
use crate::{
    api::{CompileOptionsOverride, Engine, EngineOptions},
    types::manager::TypeManager,
    values::{binder::Binder, dynamic::Value},
};
use bumpalo::Bump;

#[test]
fn test_int_package_builds() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let int_pkg = build_int_package(&arena, type_mgr).unwrap();
    let record = int_pkg.as_record().unwrap();

    // Should have all functions
    assert!(!record.is_empty());
    assert!(record.get("Quot").is_some());
    assert!(record.get("Rem").is_some());
    assert!(record.get("Div").is_some());
    assert!(record.get("Mod").is_some());
}

// Helper function for integration tests using the Engine to evaluate Melbi code
fn test_int_expr<F>(source: &str, check: F)
where
    F: FnOnce(Value),
{
    let options = EngineOptions::default();
    let arena = Bump::new();

    let engine = Engine::new(options, &arena, |arena, type_mgr, env| {
        let int_pkg = build_int_package(arena, type_mgr).unwrap();
        env.bind("Int", int_pkg)
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

// ============================================================================
// Truncated Division (Int.Quot)
// ============================================================================

#[test]
fn test_int_quot_positive_positive() {
    // 7 / 3 = 2 (truncated)
    test_int_expr("Int.Quot(7, 3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 2);
    });

    // Exact division
    test_int_expr("Int.Quot(6, 3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 2);
    });
}

#[test]
fn test_int_quot_negative_positive() {
    // -7 / 3 = -2 (truncated towards zero)
    test_int_expr("Int.Quot(-7, 3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), -2);
    });
}

#[test]
fn test_int_quot_positive_negative() {
    // 7 / -3 = -2 (truncated towards zero)
    test_int_expr("Int.Quot(7, -3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), -2);
    });
}

#[test]
fn test_int_quot_negative_negative() {
    // -7 / -3 = 2 (truncated towards zero)
    test_int_expr("Int.Quot(-7, -3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 2);
    });
}

#[test]
fn test_int_quot_zero_dividend() {
    test_int_expr("Int.Quot(0, 5)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });
}

// ============================================================================
// Truncated Remainder (Int.Rem)
// ============================================================================

#[test]
fn test_int_rem_positive_positive() {
    // 7 % 3 = 1
    test_int_expr("Int.Rem(7, 3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 1);
    });

    // Exact division: remainder is 0
    test_int_expr("Int.Rem(6, 3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });
}

#[test]
fn test_int_rem_negative_positive() {
    // -7 % 3 = -1 (sign matches dividend)
    test_int_expr("Int.Rem(-7, 3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), -1);
    });
}

#[test]
fn test_int_rem_positive_negative() {
    // 7 % -3 = 1 (sign matches dividend)
    test_int_expr("Int.Rem(7, -3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 1);
    });
}

#[test]
fn test_int_rem_negative_negative() {
    // -7 % -3 = -1 (sign matches dividend)
    test_int_expr("Int.Rem(-7, -3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), -1);
    });
}

#[test]
fn test_int_rem_zero_dividend() {
    test_int_expr("Int.Rem(0, 5)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });
}

// ============================================================================
// Euclidean Division (Int.Div)
// ============================================================================

#[test]
fn test_int_div_positive_positive() {
    // Same as truncated for positive numbers
    test_int_expr("Int.Div(7, 3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 2);
    });
}

#[test]
fn test_int_div_negative_positive() {
    // -7 div 3 = -3 (so that mod is non-negative: -7 = -3*3 + 2)
    test_int_expr("Int.Div(-7, 3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), -3);
    });
}

#[test]
fn test_int_div_positive_negative() {
    // 7 div -3 = -2 (so that mod is non-negative: 7 = -2*(-3) + 1)
    test_int_expr("Int.Div(7, -3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), -2);
    });
}

#[test]
fn test_int_div_negative_negative() {
    // -7 div -3 = 3 (so that mod is non-negative: -7 = 3*(-3) + 2)
    test_int_expr("Int.Div(-7, -3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 3);
    });
}

#[test]
fn test_int_div_zero_dividend() {
    test_int_expr("Int.Div(0, 5)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });
}

// ============================================================================
// Euclidean Modulus (Int.Mod)
// ============================================================================

#[test]
fn test_int_mod_positive_positive() {
    // Same as truncated for positive numbers
    test_int_expr("Int.Mod(7, 3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 1);
    });
}

#[test]
fn test_int_mod_negative_positive() {
    // -7 mod 3 = 2 (always non-negative)
    test_int_expr("Int.Mod(-7, 3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 2);
    });
}

#[test]
fn test_int_mod_positive_negative() {
    // 7 mod -3 = 1 (always non-negative)
    test_int_expr("Int.Mod(7, -3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 1);
    });
}

#[test]
fn test_int_mod_negative_negative() {
    // -7 mod -3 = 2 (always non-negative, even when b is negative)
    test_int_expr("Int.Mod(-7, -3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 2);
    });
}

#[test]
fn test_int_mod_zero_dividend() {
    test_int_expr("Int.Mod(0, 5)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });
}

// ============================================================================
// Invariant Tests
// ============================================================================

/// Verify the invariant: a == (Int.Quot(a, b) * b) + Int.Rem(a, b)
#[test]
fn test_truncated_division_invariant() {
    // Test multiple combinations
    let test_cases: [(i64, i64); 8] = [
        (7, 3),
        (-7, 3),
        (7, -3),
        (-7, -3),
        (10, 4),
        (-10, 4),
        (0, 5),
        (100, 7),
    ];

    for (a, b) in test_cases {
        let source = format!("(Int.Quot({a}, {b}) * {b}) + Int.Rem({a}, {b})");
        test_int_expr(&source, |r: Value| {
            assert_eq!(
                r.as_int().unwrap(),
                a,
                "Invariant failed for Quot/Rem with a={a}, b={b}"
            );
        });
    }
}

/// Verify the invariant: a == (Int.Div(a, b) * b) + Int.Mod(a, b)
#[test]
fn test_euclidean_division_invariant() {
    // Test multiple combinations
    let test_cases: [(i64, i64); 8] = [
        (7, 3),
        (-7, 3),
        (7, -3),
        (-7, -3),
        (10, 4),
        (-10, 4),
        (0, 5),
        (100, 7),
    ];

    for (a, b) in test_cases {
        let source = format!("(Int.Div({a}, {b}) * {b}) + Int.Mod({a}, {b})");
        test_int_expr(&source, |r: Value| {
            assert_eq!(
                r.as_int().unwrap(),
                a,
                "Invariant failed for Div/Mod with a={a}, b={b}"
            );
        });
    }
}

/// Verify that Int.Mod always returns non-negative values
#[test]
fn test_mod_always_non_negative() {
    let test_cases: [(i64, i64); 6] = [(-7, 3), (-7, -3), (-100, 7), (-1, 5), (-999, 13), (7, -3)];

    for (a, b) in test_cases {
        let source = format!("Int.Mod({a}, {b})");
        test_int_expr(&source, |r: Value| {
            let result = r.as_int().unwrap();
            assert!(
                result >= 0,
                "Int.Mod({a}, {b}) = {result}, expected non-negative"
            );
            assert!(
                result < b.abs(),
                "Int.Mod({a}, {b}) = {result}, expected < |{b}|"
            );
        });
    }
}

// ============================================================================
// Practical Use Cases
// ============================================================================

#[test]
fn test_mod_for_array_indexing() {
    // Using Int.Mod for circular array access with negative indices
    // If array has 5 elements, index -1 should map to 4
    test_int_expr("Int.Mod(-1, 5)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 4);
    });

    test_int_expr("Int.Mod(-6, 5)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 4);
    });
}

#[test]
fn test_quot_rem_for_digit_extraction() {
    // Extract last digit of -123 (should be -3 with truncated division)
    test_int_expr("Int.Rem(-123, 10)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), -3);
    });

    // Remove last digit of -123
    test_int_expr("Int.Quot(-123, 10)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), -12);
    });
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_division_by_one() {
    test_int_expr("Int.Quot(42, 1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 42);
    });

    test_int_expr("Int.Rem(42, 1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });

    test_int_expr("Int.Div(42, 1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 42);
    });

    test_int_expr("Int.Mod(42, 1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });
}

#[test]
fn test_large_numbers() {
    // Test with larger numbers to ensure no overflow in intermediate calculations
    test_int_expr("Int.Quot(1000000007, 1000000)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 1000);
    });

    test_int_expr("Int.Rem(1000000007, 1000000)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 7);
    });
}

// ============================================================================
// BUG-HUNTING TESTS: Division by Zero
// ============================================================================
//
// These tests probe for panics when dividing by zero. The underlying Rust
// operators (/, %, div_euclid, rem_euclid) all panic on division by zero.
// A robust implementation should return a RuntimeError::DivisionByZero.

/// Helper to test expressions that should result in a runtime error.
/// Returns Ok(()) if the expected error occurs, panics otherwise.
fn test_int_expr_expects_error(source: &str, expected_error_substring: &str) {
    use crate::api::{CompileOptionsOverride, Engine, EngineOptions};

    let options = EngineOptions::default();
    let arena = Bump::new();

    let engine = Engine::new(options, &arena, |arena, type_mgr, env| {
        let int_pkg = build_int_package(arena, type_mgr).unwrap();
        env.bind("Int", int_pkg)
    });

    let compile_opts = CompileOptionsOverride::default();
    let expr = engine
        .compile(compile_opts, source, &[])
        .expect("compilation should succeed");

    let val_arena = Bump::new();
    let result = expr.run(Default::default(), &val_arena, &[]);

    match result {
        Ok(val) => {
            panic!(
                "Expected error containing '{}', but got success with value: {:?}",
                expected_error_substring, val
            );
        }
        Err(e) => {
            let error_msg = format!("{:?}", e);
            assert!(
                error_msg.contains(expected_error_substring),
                "Expected error containing '{}', but got: {}",
                expected_error_substring,
                error_msg
            );
        }
    }
}

/// Helper to test expressions that might panic (for documenting known bugs).
/// Returns true if the expression panics, false if it succeeds or returns an error.
#[allow(dead_code)]
fn test_int_expr_panics(source: &str) -> bool {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    catch_unwind(AssertUnwindSafe(|| {
        use crate::api::{CompileOptionsOverride, Engine, EngineOptions};

        let options = EngineOptions::default();
        let arena = Bump::new();

        let engine = Engine::new(options, &arena, |arena, type_mgr, env| {
            let int_pkg = build_int_package(arena, type_mgr).unwrap();
            env.bind("Int", int_pkg)
        });

        let compile_opts = CompileOptionsOverride::default();
        let expr = engine
            .compile(compile_opts, source, &[])
            .expect("compilation should succeed");

        let val_arena = Bump::new();
        let _ = expr.run(Default::default(), &val_arena, &[]);
    }))
    .is_err()
}

#[test]
fn test_quot_division_by_zero_returns_error() {
    // Int.Quot(a, 0) returns a DivisionByZero error
    test_int_expr_expects_error("Int.Quot(7, 0)", "Division by zero");
}

#[test]
fn test_rem_division_by_zero_returns_error() {
    // Int.Rem(a, 0) returns a DivisionByZero error
    test_int_expr_expects_error("Int.Rem(7, 0)", "Division by zero");
}

#[test]
fn test_div_euclid_division_by_zero_returns_error() {
    // Int.Div(a, 0) returns a DivisionByZero error
    test_int_expr_expects_error("Int.Div(7, 0)", "Division by zero");
}

#[test]
fn test_mod_euclid_division_by_zero_returns_error() {
    // Int.Mod(a, 0) returns a DivisionByZero error
    test_int_expr_expects_error("Int.Mod(7, 0)", "Division by zero");
}

#[test]
fn test_division_by_zero_with_negative_dividend() {
    // Division by zero with negative dividend also returns DivisionByZero error
    test_int_expr_expects_error("Int.Quot(-7, 0)", "Division by zero");
    test_int_expr_expects_error("Int.Rem(-7, 0)", "Division by zero");
}

#[test]
fn test_division_by_zero_with_zero_dividend() {
    // Even 0 / 0 returns DivisionByZero error (undefined in mathematics)
    test_int_expr_expects_error("Int.Quot(0, 0)", "Division by zero");
    test_int_expr_expects_error("Int.Mod(0, 0)", "Division by zero");
}

// ============================================================================
// Integer Overflow Tests (i64::MIN / -1)
// ============================================================================
//
// This is the notorious "negative most" overflow case. In two's complement:
// - i64::MIN = -9223372036854775808
// - -i64::MIN would be 9223372036854775808, which exceeds i64::MAX
// - Therefore i64::MIN / -1 overflows (result cannot be represented)
//
// All Int functions now properly return IntegerOverflow errors for this case.

#[test]
fn test_quot_i64_min_divided_by_negative_one_returns_overflow_error() {
    // i64::MIN / -1 causes overflow - returns IntegerOverflow error
    test_int_expr_expects_error("Int.Quot(-9223372036854775808, -1)", "Integer overflow");
}

#[test]
fn test_rem_i64_min_divided_by_negative_one_returns_overflow_error() {
    // i64::MIN % -1 causes overflow during computation - returns IntegerOverflow error
    // (The mathematical result would be 0, but the operation overflows internally)
    test_int_expr_expects_error("Int.Rem(-9223372036854775808, -1)", "Integer overflow");
}

#[test]
fn test_div_euclid_i64_min_divided_by_negative_one_returns_overflow_error() {
    // i64::MIN.div_euclid(-1) causes overflow - returns IntegerOverflow error
    test_int_expr_expects_error("Int.Div(-9223372036854775808, -1)", "Integer overflow");
}

#[test]
fn test_mod_euclid_i64_min_divided_by_negative_one_returns_overflow_error() {
    // i64::MIN.rem_euclid(-1) causes overflow - returns IntegerOverflow error
    test_int_expr_expects_error("Int.Mod(-9223372036854775808, -1)", "Integer overflow");
}

// ============================================================================
// BUG-HUNTING TESTS: i64::MIN and i64::MAX Edge Cases
// ============================================================================

#[test]
fn test_i64_max_operations() {
    // i64::MAX = 9223372036854775807
    // These should work correctly without overflow

    // i64::MAX / 2 = 4611686018427387903
    test_int_expr("Int.Quot(9223372036854775807, 2)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 4611686018427387903);
    });

    // i64::MAX % 2 = 1 (i64::MAX is odd)
    test_int_expr("Int.Rem(9223372036854775807, 2)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 1);
    });

    // i64::MAX / 1 = i64::MAX
    test_int_expr("Int.Quot(9223372036854775807, 1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), i64::MAX);
    });

    // i64::MAX % 1 = 0
    test_int_expr("Int.Rem(9223372036854775807, 1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });
}

#[test]
fn test_i64_min_safe_operations() {
    // i64::MIN = -9223372036854775808
    // These operations should be safe (no overflow)

    // i64::MIN / 2 = -4611686018427387904
    test_int_expr("Int.Quot(-9223372036854775808, 2)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), -4611686018427387904);
    });

    // i64::MIN % 2 = 0 (i64::MIN is even in two's complement)
    test_int_expr("Int.Rem(-9223372036854775808, 2)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });

    // i64::MIN / 1 = i64::MIN
    test_int_expr("Int.Quot(-9223372036854775808, 1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), i64::MIN);
    });

    // i64::MIN % 1 = 0
    test_int_expr("Int.Rem(-9223372036854775808, 1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });
}

#[test]
fn test_i64_max_divided_by_negative_one() {
    // i64::MAX / -1 = -i64::MAX = -9223372036854775807 (fits in i64)
    test_int_expr("Int.Quot(9223372036854775807, -1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), -9223372036854775807);
    });

    // i64::MAX % -1 = 0
    test_int_expr("Int.Rem(9223372036854775807, -1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });
}

#[test]
fn test_i64_min_divided_by_i64_max() {
    // i64::MIN / i64::MAX = -1 (truncated towards zero: -1.0000...)
    test_int_expr(
        "Int.Quot(-9223372036854775808, 9223372036854775807)",
        |r: Value| {
            assert_eq!(r.as_int().unwrap(), -1);
        },
    );

    // i64::MIN % i64::MAX = -1
    // -9223372036854775808 = -1 * 9223372036854775807 + (-1)
    test_int_expr(
        "Int.Rem(-9223372036854775808, 9223372036854775807)",
        |r: Value| {
            assert_eq!(r.as_int().unwrap(), -1);
        },
    );
}

#[test]
fn test_i64_max_divided_by_i64_min() {
    // i64::MAX / i64::MIN = 0 (truncated towards zero)
    test_int_expr(
        "Int.Quot(9223372036854775807, -9223372036854775808)",
        |r: Value| {
            assert_eq!(r.as_int().unwrap(), 0);
        },
    );

    // i64::MAX % i64::MIN = i64::MAX (since quotient is 0)
    test_int_expr(
        "Int.Rem(9223372036854775807, -9223372036854775808)",
        |r: Value| {
            assert_eq!(r.as_int().unwrap(), i64::MAX);
        },
    );
}

#[test]
fn test_i64_min_divided_by_i64_min() {
    // i64::MIN / i64::MIN = 1
    test_int_expr(
        "Int.Quot(-9223372036854775808, -9223372036854775808)",
        |r: Value| {
            assert_eq!(r.as_int().unwrap(), 1);
        },
    );

    // i64::MIN % i64::MIN = 0
    test_int_expr(
        "Int.Rem(-9223372036854775808, -9223372036854775808)",
        |r: Value| {
            assert_eq!(r.as_int().unwrap(), 0);
        },
    );
}

#[test]
fn test_i64_max_divided_by_i64_max() {
    // i64::MAX / i64::MAX = 1
    test_int_expr(
        "Int.Quot(9223372036854775807, 9223372036854775807)",
        |r: Value| {
            assert_eq!(r.as_int().unwrap(), 1);
        },
    );

    // i64::MAX % i64::MAX = 0
    test_int_expr(
        "Int.Rem(9223372036854775807, 9223372036854775807)",
        |r: Value| {
            assert_eq!(r.as_int().unwrap(), 0);
        },
    );
}

// ============================================================================
// BUG-HUNTING TESTS: Euclidean Operations with Extreme Values
// ============================================================================

#[test]
fn test_euclidean_div_i64_min_with_positive_divisor() {
    // Int.Div(-9223372036854775808, 2) should work
    test_int_expr("Int.Div(-9223372036854775808, 2)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), -4611686018427387904);
    });

    // Int.Mod(-9223372036854775808, 2) should return 0 (i64::MIN is even)
    test_int_expr("Int.Mod(-9223372036854775808, 2)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });
}

#[test]
fn test_euclidean_div_i64_min_with_large_positive_divisor() {
    // Test with divisor close to i64::MAX
    test_int_expr(
        "Int.Div(-9223372036854775808, 9223372036854775807)",
        |r: Value| {
            // Euclidean division: result should be -2 because
            // -9223372036854775808 = -2 * 9223372036854775807 + 9223372036854775806
            assert_eq!(r.as_int().unwrap(), -2);
        },
    );

    test_int_expr(
        "Int.Mod(-9223372036854775808, 9223372036854775807)",
        |r: Value| {
            // The mod should be positive: 9223372036854775806
            assert_eq!(r.as_int().unwrap(), 9223372036854775806);
        },
    );
}

// ============================================================================
// BUG-HUNTING TESTS: Boundary Values Near Zero
// ============================================================================

#[test]
fn test_small_divisors() {
    // Division by 1 and -1 are boundary cases

    // Any number / 1 = that number
    test_int_expr("Int.Quot(42, 1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 42);
    });

    test_int_expr("Int.Quot(-42, 1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), -42);
    });

    // Any number / -1 = negated number (except i64::MIN)
    test_int_expr("Int.Quot(42, -1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), -42);
    });

    test_int_expr("Int.Quot(-42, -1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 42);
    });

    // Any number % 1 or % -1 = 0
    test_int_expr("Int.Rem(42, 1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });

    test_int_expr("Int.Rem(-42, -1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });
}

#[test]
fn test_divisor_larger_than_dividend() {
    // When |divisor| > |dividend|, truncated division returns 0
    test_int_expr("Int.Quot(5, 100)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });

    test_int_expr("Int.Quot(-5, 100)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });

    // Remainder equals dividend when |divisor| > |dividend|
    test_int_expr("Int.Rem(5, 100)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 5);
    });

    test_int_expr("Int.Rem(-5, 100)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), -5);
    });
}

#[test]
fn test_euclidean_divisor_larger_than_dividend() {
    // Euclidean division with larger divisor

    // Positive dividend: same as truncated
    test_int_expr("Int.Div(5, 100)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });

    test_int_expr("Int.Mod(5, 100)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 5);
    });

    // Negative dividend: Euclidean rounds towards negative infinity
    // -5 = -1 * 100 + 95, so Div = -1, Mod = 95
    test_int_expr("Int.Div(-5, 100)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), -1);
    });

    test_int_expr("Int.Mod(-5, 100)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 95);
    });
}

// ============================================================================
// BUG-HUNTING TESTS: Invariant Verification with Edge Cases
// ============================================================================

#[test]
fn test_truncated_invariant_with_extreme_values() {
    // Verify a == (a / b) * b + (a % b) for extreme values
    // (Skipping i64::MIN / -1 cases which overflow)

    // i64::MAX with various divisors
    let a = i64::MAX;
    for b in [2i64, 3, 7, 1000, i64::MAX] {
        let source = format!("(Int.Quot({a}, {b}) * {b}) + Int.Rem({a}, {b})");
        test_int_expr(&source, |r: Value| {
            assert_eq!(
                r.as_int().unwrap(),
                a,
                "Invariant failed for Quot/Rem with a={a}, b={b}"
            );
        });
    }

    // i64::MIN with safe divisors (not -1)
    let a = i64::MIN;
    for b in [2i64, 3, 7, 1000, i64::MAX] {
        let source = format!("(Int.Quot({a}, {b}) * {b}) + Int.Rem({a}, {b})");
        test_int_expr(&source, |r: Value| {
            assert_eq!(
                r.as_int().unwrap(),
                a,
                "Invariant failed for Quot/Rem with a={a}, b={b}"
            );
        });
    }
}

#[test]
fn test_euclidean_invariant_with_extreme_values() {
    // Verify a == (a div b) * b + (a mod b) for extreme values
    // (Skipping i64::MIN / -1 cases which overflow)

    // i64::MAX with various divisors
    let a = i64::MAX;
    for b in [2i64, 3, 7, 1000, i64::MAX] {
        let source = format!("(Int.Div({a}, {b}) * {b}) + Int.Mod({a}, {b})");
        test_int_expr(&source, |r: Value| {
            assert_eq!(
                r.as_int().unwrap(),
                a,
                "Invariant failed for Div/Mod with a={a}, b={b}"
            );
        });
    }

    // i64::MIN with safe divisors (not -1)
    let a = i64::MIN;
    for b in [2i64, 3, 7, 1000, i64::MAX] {
        let source = format!("(Int.Div({a}, {b}) * {b}) + Int.Mod({a}, {b})");
        test_int_expr(&source, |r: Value| {
            assert_eq!(
                r.as_int().unwrap(),
                a,
                "Invariant failed for Div/Mod with a={a}, b={b}"
            );
        });
    }
}

// ============================================================================
// BUG-HUNTING TESTS: Powers of Two (Common Bug Triggers)
// ============================================================================

#[test]
fn test_powers_of_two_divisors() {
    // Powers of two are common divisors and can trigger off-by-one bugs

    // 2^62 as divisor (largest power of 2 that fits cleanly)
    test_int_expr(
        "Int.Quot(9223372036854775807, 4611686018427387904)",
        |r: Value| {
            // i64::MAX / 2^62 = 1 (truncated)
            assert_eq!(r.as_int().unwrap(), 1);
        },
    );

    test_int_expr(
        "Int.Rem(9223372036854775807, 4611686018427387904)",
        |r: Value| {
            // i64::MAX % 2^62 = i64::MAX - 2^62 = 4611686018427387903
            assert_eq!(r.as_int().unwrap(), 4611686018427387903);
        },
    );
}

#[test]
fn test_negative_dividend_power_of_two_divisor() {
    // Negative dividend with power of two divisor
    // This is where truncated and Euclidean division differ

    // -7 with divisor 4
    test_int_expr("Int.Quot(-7, 4)", |r: Value| {
        // Truncated: -7 / 4 = -1 (rounds towards zero)
        assert_eq!(r.as_int().unwrap(), -1);
    });

    test_int_expr("Int.Rem(-7, 4)", |r: Value| {
        // Truncated: -7 % 4 = -3
        assert_eq!(r.as_int().unwrap(), -3);
    });

    test_int_expr("Int.Div(-7, 4)", |r: Value| {
        // Euclidean: -7 div 4 = -2 (so that mod is non-negative)
        assert_eq!(r.as_int().unwrap(), -2);
    });

    test_int_expr("Int.Mod(-7, 4)", |r: Value| {
        // Euclidean: -7 mod 4 = 1 (always non-negative)
        assert_eq!(r.as_int().unwrap(), 1);
    });
}

// ============================================================================
// BUG-HUNTING TESTS: Negative Divisors with Euclidean Operations
// ============================================================================

#[test]
fn test_euclidean_with_negative_divisors() {
    // Euclidean division with negative divisors has subtle behavior
    // The quotient is adjusted so that remainder is always in [0, |divisor|)

    // 7 with divisor -3
    test_int_expr("Int.Div(7, -3)", |r: Value| {
        // 7 = -2 * (-3) + 1, so Div = -2, Mod = 1
        assert_eq!(r.as_int().unwrap(), -2);
    });

    test_int_expr("Int.Mod(7, -3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 1);
    });

    // -7 with divisor -3
    test_int_expr("Int.Div(-7, -3)", |r: Value| {
        // -7 = 3 * (-3) + 2, so Div = 3, Mod = 2
        assert_eq!(r.as_int().unwrap(), 3);
    });

    test_int_expr("Int.Mod(-7, -3)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 2);
    });
}

#[test]
fn test_euclidean_with_large_negative_divisor() {
    // Test with i64::MIN as divisor (but not i64::MIN/-1 case)

    // i64::MAX with divisor i64::MIN
    test_int_expr(
        "Int.Div(9223372036854775807, -9223372036854775808)",
        |r: Value| {
            // 9223372036854775807 div -9223372036854775808 = 0
            // because |divisor| > |dividend| for positive dividend
            assert_eq!(r.as_int().unwrap(), 0);
        },
    );

    test_int_expr(
        "Int.Mod(9223372036854775807, -9223372036854775808)",
        |r: Value| {
            // Mod = i64::MAX (since quotient is 0)
            assert_eq!(r.as_int().unwrap(), i64::MAX);
        },
    );
}

// ============================================================================
// BUG-HUNTING TESTS: Values Near i64::MIN That Aren't i64::MIN
// ============================================================================

#[test]
fn test_values_near_i64_min() {
    // Test i64::MIN + 1 which should not have the overflow issue

    // (i64::MIN + 1) / -1 should work without overflow
    // -9223372036854775807 / -1 = 9223372036854775807 = i64::MAX
    test_int_expr("Int.Quot(-9223372036854775807, -1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), i64::MAX);
    });

    test_int_expr("Int.Rem(-9223372036854775807, -1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });

    // Euclidean versions should also work
    test_int_expr("Int.Div(-9223372036854775807, -1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), i64::MAX);
    });

    test_int_expr("Int.Mod(-9223372036854775807, -1)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 0);
    });
}

// ============================================================================
// BUG-HUNTING TESTS: Consistency Between Truncated and Euclidean
// ============================================================================

#[test]
fn test_truncated_vs_euclidean_positive_inputs() {
    // For positive dividend and divisor, truncated and Euclidean should match
    let test_cases = [(7, 3), (100, 7), (1000000, 999), (i64::MAX, 2)];

    for (a, b) in test_cases {
        let quot_source = format!("Int.Quot({a}, {b})");
        let div_source = format!("Int.Div({a}, {b})");
        let rem_source = format!("Int.Rem({a}, {b})");
        let mod_source = format!("Int.Mod({a}, {b})");

        let mut quot_result = 0i64;
        let mut div_result = 0i64;
        let mut rem_result = 0i64;
        let mut mod_result = 0i64;

        test_int_expr(&quot_source, |r: Value| {
            quot_result = r.as_int().unwrap();
        });
        test_int_expr(&div_source, |r: Value| {
            div_result = r.as_int().unwrap();
        });
        test_int_expr(&rem_source, |r: Value| {
            rem_result = r.as_int().unwrap();
        });
        test_int_expr(&mod_source, |r: Value| {
            mod_result = r.as_int().unwrap();
        });

        assert_eq!(
            quot_result, div_result,
            "Quot and Div should match for positive inputs: a={a}, b={b}"
        );
        assert_eq!(
            rem_result, mod_result,
            "Rem and Mod should match for positive inputs: a={a}, b={b}"
        );
    }
}

#[test]
fn test_truncated_vs_euclidean_differ_for_negative() {
    // For negative dividend and positive divisor, they should differ
    // (unless remainder would be 0)

    // -7 / 3: Quot = -2, Rem = -1
    //         Div = -3, Mod = 2
    let mut quot_result = 0i64;
    let mut div_result = 0i64;

    test_int_expr("Int.Quot(-7, 3)", |r: Value| {
        quot_result = r.as_int().unwrap();
    });
    test_int_expr("Int.Div(-7, 3)", |r: Value| {
        div_result = r.as_int().unwrap();
    });

    assert_ne!(
        quot_result, div_result,
        "Quot and Div should differ for negative dividend with positive divisor"
    );
    assert_eq!(quot_result, -2);
    assert_eq!(div_result, -3);
}
