#![allow(non_upper_case_globals)]
#![allow(dead_code)]
//! Tests for the `#[melbi_fn_new]` proc macro
//!
//! These tests use a mock `melbi_fn_generate!` macro that stringifies its arguments,
//! allowing us to verify the proc macro's output without needing the full implementation.

use melbi_macros::melbi_fn_new;

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
        context = $context:ident,
        signature = $sig:tt -> $ret_ty:ty,
        fallible = $fallible:tt
    ) => {
        const $name: &'static str = stringify!(
            fn_name = $fn_name,
            lt = $lt,
            context = $context,
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

    #[melbi_fn_new]
    fn add(a: i64, b: i64) -> i64 {
        a + b
    }

    #[test]
    fn test_pure_two_params() {
        // Normalize whitespace for comparison
        let normalized: String = Add.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            normalized,
            "fn_name = add, lt = '__melbi, context = Pure, signature = { a : i64, b : i64 } -> i64, fallible = false"
        );
    }

    struct Bump;
    struct TypeManager;

    #[melbi_fn_new]
    fn legacy_div(_arena: &Bump, _type_mgr: &TypeManager, a: i64, b: i64) -> i64 {
        a / b
    }

    #[test]
    fn test_legacy_two_params() {
        let normalized: String = LegacyDiv.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            normalized,
            "fn_name = legacy_div, lt = '__melbi, context = Legacy, signature = { a : i64, b : i64 } -> i64, fallible = false"
        );
    }

    struct RuntimeError;

    #[melbi_fn_new]
    fn safe_div(a: i64, b: i64) -> Result<i64, RuntimeError> {
        if b == 0 { Err(RuntimeError) } else { Ok(a / b) }
    }

    #[test]
    fn test_fallible_pure() {
        let normalized: String = SafeDiv.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            normalized,
            "fn_name = safe_div, lt = '__melbi, context = Pure, signature = { a : i64, b : i64 } -> i64, fallible = true"
        );
    }

    struct Str<'a>(&'a str);

    #[melbi_fn_new]
    fn upper<'a>(s: Str<'a>) -> Str<'a> {
        s
    }

    #[test]
    fn test_with_lifetime() {
        let normalized: String = Upper.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            normalized,
            "fn_name = upper, lt = 'a, context = Pure, signature = { s : Str < 'a > } -> Str < 'a >, fallible = false"
        );
    }

    #[melbi_fn_new]
    fn no_params() -> i64 {
        42
    }

    #[test]
    fn test_zero_params() {
        let normalized: String = NoParams.split_whitespace().collect::<Vec<_>>().join(" ");
        assert_eq!(
            normalized,
            "fn_name = no_params, lt = '__melbi, context = Pure, signature = {} -> i64, fallible = false"
        );
    }
}

// ============================================================================
// Name Derivation Tests
// ============================================================================

mod name_derivation {
    use super::*;

    #[melbi_fn_new]
    fn simple_add(a: i64, b: i64) -> i64 {
        a + b
    }

    #[test]
    fn test_simple_name_to_pascal_case() {
        // simple_add -> SimpleAdd
        assert_output_contains!(SimpleAdd, "fn_name = simple_add");
    }

    #[melbi_fn_new]
    fn add_two_numbers(a: i64, b: i64) -> i64 {
        a + b
    }

    #[test]
    fn test_multi_word_name_to_pascal_case() {
        // add_two_numbers -> AddTwoNumbers
        assert_output_contains!(AddTwoNumbers, "fn_name = add_two_numbers");
    }

    #[melbi_fn_new(name = "CustomName")]
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

    // Mock types for testing context detection
    struct Bump;
    struct TypeManager;
    struct FfiContext;

    #[melbi_fn_new]
    fn pure_function(a: i64, b: i64) -> i64 {
        a + b
    }

    #[test]
    fn test_pure_mode() {
        assert_output_contains!(PureFunction, "context = Pure");
    }

    #[melbi_fn_new]
    fn arena_only_function(arena: &Bump, x: i64) -> i64 {
        let _ = arena;
        x
    }

    #[test]
    fn test_arena_only_mode() {
        assert_output_contains!(ArenaOnlyFunction, "context = ArenaOnly");
    }

    #[melbi_fn_new]
    fn arena_only_with_underscore(_arena: &Bump, x: i64) -> i64 {
        x
    }

    #[test]
    fn test_arena_only_with_underscore() {
        assert_output_contains!(ArenaOnlyWithUnderscore, "context = ArenaOnly");
    }

    #[melbi_fn_new]
    fn type_mgr_only_function(type_mgr: &TypeManager, x: i64) -> i64 {
        let _ = type_mgr;
        x
    }

    #[test]
    fn test_type_mgr_only_mode() {
        assert_output_contains!(TypeMgrOnlyFunction, "context = TypeMgrOnly");
    }

    #[melbi_fn_new]
    fn type_mgr_only_with_underscore(_type_mgr: &TypeManager, x: i64) -> i64 {
        x
    }

    #[test]
    fn test_type_mgr_only_with_underscore() {
        assert_output_contains!(TypeMgrOnlyWithUnderscore, "context = TypeMgrOnly");
    }

    #[melbi_fn_new]
    fn legacy_function(arena: &Bump, type_mgr: &TypeManager, x: i64) -> i64 {
        let _ = (arena, type_mgr);
        x
    }

    #[test]
    fn test_legacy_mode() {
        assert_output_contains!(LegacyFunction, "context = Legacy");
    }

    #[melbi_fn_new]
    fn legacy_with_underscores(_arena: &Bump, _type_mgr: &TypeManager, x: i64) -> i64 {
        x
    }

    #[test]
    fn test_legacy_with_underscores() {
        assert_output_contains!(LegacyWithUnderscores, "context = Legacy");
    }

    #[melbi_fn_new]
    fn full_context_function(ctx: &FfiContext, x: i64) -> i64 {
        let _ = ctx;
        x
    }

    #[test]
    fn test_full_context_mode() {
        assert_output_contains!(FullContextFunction, "context = FullContext");
    }

    #[melbi_fn_new]
    fn zero_args_pure() -> i64 {
        42
    }

    #[test]
    fn test_zero_args_is_pure() {
        assert_output_contains!(ZeroArgsPure, "context = Pure");
    }

    #[melbi_fn_new]
    fn zero_args_legacy(_arena: &Bump, _type_mgr: &TypeManager) -> i64 {
        42
    }

    #[test]
    fn test_zero_business_args_legacy() {
        assert_output_contains!(ZeroArgsLegacy, "context = Legacy");
    }
}

// ============================================================================
// Fallible Detection Tests
// ============================================================================

mod fallible {
    use super::*;

    struct RuntimeError;

    #[melbi_fn_new]
    fn infallible_function(a: i64) -> i64 {
        a
    }

    #[test]
    fn test_infallible() {
        assert_output_contains!(InfallibleFunction, "fallible = false");
    }

    #[melbi_fn_new]
    fn fallible_function(a: i64) -> Result<i64, RuntimeError> {
        Ok(a)
    }

    #[test]
    fn test_fallible() {
        assert_output_contains!(FallibleFunction, "fallible = true");
    }

    #[melbi_fn_new]
    fn fallible_with_unwrapped_return(a: i64, b: i64) -> Result<i64, RuntimeError> {
        if b == 0 { Err(RuntimeError) } else { Ok(a / b) }
    }

    #[test]
    fn test_fallible_unwraps_return_type() {
        // The bridge return type should be i64, not Result<i64, RuntimeError>
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

    #[melbi_fn_new]
    fn no_lifetime_uses_default(x: i64) -> i64 {
        x
    }

    #[test]
    fn test_default_lifetime() {
        assert_output_contains!(NoLifetimeUsesDefault, "lt = '__melbi");
    }

    #[melbi_fn_new]
    fn with_lifetime<'a>(s: Str<'a>) -> Str<'a> {
        s
    }

    #[test]
    fn test_explicit_lifetime() {
        assert_output_contains!(WithLifetime, "lt = 'a");
    }

    #[melbi_fn_new]
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

    #[melbi_fn_new]
    fn no_params() -> i64 {
        42
    }

    #[test]
    fn test_no_params_signature() {
        // signature should have empty params: {} -> i64 (no spaces in empty braces)
        assert_output_contains!(NoParams, "signature = {} -> i64");
    }

    #[melbi_fn_new]
    fn single_param(x: i64) -> i64 {
        x
    }

    #[test]
    fn test_single_param_signature() {
        assert_output_contains!(SingleParam, "x : i64");
    }

    #[melbi_fn_new]
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

    // Test that context params are NOT included in signature
    struct Bump;
    struct TypeManager;

    #[melbi_fn_new]
    fn legacy_with_business_params(_arena: &Bump, _type_mgr: &TypeManager, x: i64, y: i64) -> i64 {
        x + y
    }

    #[test]
    fn test_context_params_excluded_from_signature() {
        // Should contain x and y but not arena or type_mgr
        assert_output_contains!(LegacyWithBusinessParams, "x : i64");
        assert_output_contains!(LegacyWithBusinessParams, "y : i64");
        assert_output_not_contains!(LegacyWithBusinessParams, "arena");
        assert_output_not_contains!(LegacyWithBusinessParams, "type_mgr");
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

    #[melbi_fn_new]
    fn with_array<'a>(arr: Array<'a, i64>) -> i64 {
        let _ = arr;
        0
    }

    #[test]
    fn test_array_param() {
        assert_output_contains!(WithArray, "arr : Array");
    }

    #[melbi_fn_new]
    fn with_optional<'a>(opt: Optional<'a, i64>) -> i64 {
        let _ = opt;
        0
    }

    #[test]
    fn test_optional_param() {
        assert_output_contains!(WithOptional, "opt : Optional");
    }

    #[melbi_fn_new]
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

    #[melbi_fn_new]
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
    #[melbi_fn_new]
    fn a() -> i64 {
        0
    }

    #[test]
    fn test_single_letter_name() {
        assert_output_contains!(A, "fn_name = a");
    }

    // Function name starting with underscore
    #[melbi_fn_new]
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
    #[melbi_fn_new]
    fn foo__bar() -> i64 {
        0
    }

    #[test]
    fn test_double_underscore() {
        // foo__bar -> FooBar (consecutive underscores collapse)
        assert_output_contains!(FooBar, "fn_name = foo__bar");
    }
}
