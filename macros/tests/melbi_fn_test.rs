#![allow(non_upper_case_globals)]
#![allow(dead_code)]
//! Tests for the `#[melbi_fn]` proc macro
//!
//! These tests use a mock `melbi_fn_generate!` macro that stringifies its arguments,
//! allowing us to verify the proc macro's output without needing the full implementation.

use melbi_macros::melbi_fn;

// ============================================================================
// Mock Declarative Macro
// ============================================================================

/// Mock implementation that captures the generated arguments as a const string.
/// This lets us verify what the proc macro generates.
macro_rules! melbi_fn_generate {
    (
        name = $name:ident,
        fn_name = $fn_name:ident,
        lt = $lt:lifetime,
        context_arg = $context_arg:tt,
        signature = $sig:tt -> $ret_ty:ty,
        fallible = $fallible:tt
    ) => {
        const $name: &'static str = stringify!(
            fn_name = $fn_name,
            lt = $lt,
            context_arg = $context_arg,
            signature = $sig -> $ret_ty,
            fallible = $fallible
        );
    };
}

// ============================================================================
// Test Helpers
// ============================================================================

/// Assert that `haystack` contains `needle`, showing the full output on failure.
macro_rules! assert_output_contains {
    ($haystack:expr, $needle:expr) => {
        assert!(
            $haystack.contains($needle),
            "expected output to contain {:?}\n\nactual output:\n{}",
            $needle,
            $haystack
        );
    };
}

/// Assert that `haystack` does NOT contain `needle`, showing the full output on failure.
macro_rules! assert_output_not_contains {
    ($haystack:expr, $needle:expr) => {
        assert!(
            !$haystack.contains($needle),
            "expected output to NOT contain {:?}\n\nactual output:\n{}",
            $needle,
            $haystack
        );
    };
}

// ============================================================================
// Full Output Comparison Tests
// ============================================================================

/// These tests compare the complete output to catch regressions.
/// Note: stringify! adds spaces around tokens and may insert newlines.
mod full_output {
    use super::*;

    #[melbi_fn]
    fn add(a: i64, b: i64) -> i64 {
        a + b
    }

    #[test]
    fn test_pure_two_params() {
        // Normalize whitespace for comparison
        let normalized: String = Add.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            normalized,
            "fn_name = add, lt = '__a, context_arg = false, signature = { a : i64, b : i64 } -> i64, fallible = false"
        );
    }

    struct FfiContext;

    #[melbi_fn]
    fn with_context(ctx: &FfiContext, a: i64, b: i64) -> i64 {
        let _ = ctx;
        a + b
    }

    #[test]
    fn test_with_context_two_params() {
        let normalized: String = WithContext.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            normalized,
            "fn_name = with_context, lt = '__a, context_arg = true, signature = { a : i64, b : i64 } -> i64, fallible = false"
        );
    }

    struct RuntimeError;

    #[melbi_fn]
    fn safe_div(a: i64, b: i64) -> Result<i64, RuntimeError> {
        if b == 0 { Err(RuntimeError) } else { Ok(a / b) }
    }

    #[test]
    fn test_fallible_pure() {
        let normalized: String = SafeDiv.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            normalized,
            "fn_name = safe_div, lt = '__a, context_arg = false, signature = { a : i64, b : i64 } -> i64, fallible = true"
        );
    }

    struct Str<'a>(&'a str);

    #[melbi_fn]
    fn upper<'a>(s: Str<'a>) -> Str<'a> {
        s
    }

    #[test]
    fn test_with_lifetime() {
        let normalized: String = Upper.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            normalized,
            "fn_name = upper, lt = 'a, context_arg = false, signature = { s : Str < 'a > } -> Str < 'a >, fallible = false"
        );
    }

    #[melbi_fn]
    fn no_params() -> i64 {
        42
    }

    #[test]
    fn test_zero_params() {
        let normalized: String = NoParams.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            normalized,
            "fn_name = no_params, lt = '__a, context_arg = false, signature = {} -> i64, fallible = false"
        );
    }
}

// ============================================================================
// Name Derivation Tests
// ============================================================================

mod name_derivation {
    use super::*;

    #[melbi_fn]
    fn simple_add(a: i64, b: i64) -> i64 {
        a + b
    }

    #[test]
    fn test_simple_name_to_pascal_case() {
        // simple_add -> SimpleAdd
        assert_output_contains!(SimpleAdd, "fn_name = simple_add");
    }

    #[melbi_fn]
    fn add_two_numbers(a: i64, b: i64) -> i64 {
        a + b
    }

    #[test]
    fn test_multi_word_name_to_pascal_case() {
        // add_two_numbers -> AddTwoNumbers
        assert_output_contains!(AddTwoNumbers, "fn_name = add_two_numbers");
    }

    #[melbi_fn(name = "CustomName")]
    fn my_function(x: i64) -> i64 {
        x
    }

    #[test]
    fn test_explicit_name_override() {
        // Explicit name should be used
        assert_output_contains!(CustomName, "fn_name = my_function");
    }
}

// ============================================================================
// Context Mode Detection Tests
// ============================================================================

mod context_modes {
    use super::*;

    struct FfiContext;

    #[melbi_fn]
    fn pure_function(a: i64, b: i64) -> i64 {
        a + b
    }

    #[test]
    fn test_pure_mode() {
        assert_output_contains!(PureFunction, "context_arg = false");
    }

    #[melbi_fn]
    fn with_context_function(ctx: &FfiContext, x: i64) -> i64 {
        let _ = ctx;
        x
    }

    #[test]
    fn test_with_context_mode() {
        assert_output_contains!(WithContextFunction, "context_arg = true");
    }

    #[melbi_fn]
    fn zero_args_pure() -> i64 {
        42
    }

    #[test]
    fn test_zero_args_is_pure() {
        assert_output_contains!(ZeroArgsPure, "context_arg = false");
    }

    #[melbi_fn]
    fn zero_args_with_context(_ctx: &FfiContext) -> i64 {
        42
    }

    #[test]
    fn test_zero_business_args_with_context() {
        assert_output_contains!(ZeroArgsWithContext, "context_arg = true");
    }

    // Alternative parameter name for FfiContext
    #[melbi_fn]
    fn context_alt_name(context: &FfiContext, x: i64) -> i64 {
        let _ = context;
        x
    }

    #[test]
    fn test_context_alt_name() {
        assert_output_contains!(ContextAltName, "context_arg = true");
    }
}

// ============================================================================
// Fallible Detection Tests
// ============================================================================

mod fallible {
    use super::*;

    struct RuntimeError;

    #[melbi_fn]
    fn infallible_function(a: i64) -> i64 {
        a
    }

    #[test]
    fn test_infallible() {
        assert_output_contains!(InfallibleFunction, "fallible = false");
    }

    #[melbi_fn]
    fn fallible_function(a: i64) -> Result<i64, RuntimeError> {
        Ok(a)
    }

    #[test]
    fn test_fallible() {
        assert_output_contains!(FallibleFunction, "fallible = true");
    }

    #[melbi_fn]
    fn fallible_with_unwrapped_return(a: i64, b: i64) -> Result<i64, RuntimeError> {
        if b == 0 { Err(RuntimeError) } else { Ok(a / b) }
    }

    #[test]
    fn test_fallible_unwraps_return_type() {
        // The ok return type should be i64, not Result<i64, RuntimeError>
        assert_output_contains!(FallibleWithUnwrappedReturn, "-> i64");
        assert_output_not_contains!(FallibleWithUnwrappedReturn, "Result");
    }
}

// ============================================================================
// Lifetime Extraction Tests
// ============================================================================

mod lifetimes {
    use super::*;

    struct Str<'a>(&'a str);

    #[melbi_fn]
    fn no_lifetime_uses_default(x: i64) -> i64 {
        x
    }

    #[test]
    fn test_default_lifetime() {
        assert_output_contains!(NoLifetimeUsesDefault, "lt = '__a");
    }

    #[melbi_fn]
    fn with_lifetime<'a>(s: Str<'a>) -> Str<'a> {
        s
    }

    #[test]
    fn test_explicit_lifetime() {
        assert_output_contains!(WithLifetime, "lt = 'a");
    }

    #[melbi_fn]
    fn with_named_lifetime<'value>(s: Str<'value>) -> Str<'value> {
        s
    }

    #[test]
    fn test_named_lifetime() {
        assert_output_contains!(WithNamedLifetime, "lt = 'value");
    }
}

// ============================================================================
// Signature Tests
// ============================================================================

mod signature {
    use super::*;

    #[melbi_fn]
    fn no_params() -> i64 {
        42
    }

    #[test]
    fn test_no_params_signature() {
        // signature should have empty params: {} -> i64 (no spaces in empty braces)
        assert_output_contains!(NoParams, "signature = {} -> i64");
    }

    #[melbi_fn]
    fn single_param(x: i64) -> i64 {
        x
    }

    #[test]
    fn test_single_param_signature() {
        assert_output_contains!(SingleParam, "x : i64");
    }

    #[melbi_fn]
    fn multiple_params(a: i64, b: f64, c: bool) -> f64 {
        if c { a as f64 + b } else { b }
    }

    #[test]
    fn test_multiple_params_signature() {
        assert_output_contains!(MultipleParams, "a : i64");
        assert_output_contains!(MultipleParams, "b : f64");
        assert_output_contains!(MultipleParams, "c : bool");
        assert_output_contains!(MultipleParams, "-> f64");
    }

    // Test that FfiContext is NOT included in signature
    struct FfiContext;

    #[melbi_fn]
    fn with_context_and_params(_ctx: &FfiContext, x: i64, y: i64) -> i64 {
        x + y
    }

    #[test]
    fn test_context_param_excluded_from_signature() {
        // Should contain x and y but not ctx or FfiContext
        assert_output_contains!(WithContextAndParams, "x : i64");
        assert_output_contains!(WithContextAndParams, "y : i64");
        assert_output_not_contains!(WithContextAndParams, "ctx");
        assert_output_not_contains!(WithContextAndParams, "FfiContext");
    }
}

// ============================================================================
// Complex Type Tests
// ============================================================================

mod complex_types {
    use super::*;

    struct Array<'a, T>(&'a [T]);
    struct Optional<'a, T>(&'a Option<T>);
    struct Str<'a>(&'a str);

    #[melbi_fn]
    fn with_array<'a>(arr: Array<'a, i64>) -> i64 {
        let _ = arr;
        0
    }

    #[test]
    fn test_array_param() {
        assert_output_contains!(WithArray, "arr : Array");
    }

    #[melbi_fn]
    fn with_optional<'a>(opt: Optional<'a, i64>) -> i64 {
        let _ = opt;
        0
    }

    #[test]
    fn test_optional_param() {
        assert_output_contains!(WithOptional, "opt : Optional");
    }

    #[melbi_fn]
    fn with_str<'a>(s: Str<'a>) -> Str<'a> {
        s
    }

    #[test]
    fn test_str_param_and_return() {
        assert_output_contains!(WithStr, "s : Str");
        // Return type appears after -> (may have newline due to stringify!)
        // Just verify the return type info is present
        assert_output_contains!(WithStr, "Str < 'a >");
    }

    #[melbi_fn]
    fn nested_generic<'a>(arr: Array<'a, Str<'a>>) -> i64 {
        let _ = arr;
        0
    }

    #[test]
    fn test_nested_generic() {
        assert_output_contains!(NestedGeneric, "arr : Array");
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

mod edge_cases {
    use super::*;

    // Single letter function name
    #[melbi_fn]
    fn a() -> i64 {
        0
    }

    #[test]
    fn test_single_letter_name() {
        assert_output_contains!(A, "fn_name = a");
    }

    // Function name starting with underscore
    #[melbi_fn]
    fn _private_helper() -> i64 {
        0
    }

    #[test]
    fn test_underscore_prefix_name() {
        // _private_helper -> PrivateHelper
        assert_output_contains!(PrivateHelper, "fn_name = _private_helper");
    }

    // Multiple consecutive underscores
    #[allow(non_snake_case)]
    #[melbi_fn]
    fn foo__bar() -> i64 {
        0
    }

    #[test]
    fn test_double_underscore() {
        // foo__bar -> FooBar (consecutive underscores collapse)
        assert_output_contains!(FooBar, "fn_name = foo__bar");
    }
}
