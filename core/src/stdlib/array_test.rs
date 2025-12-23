//! Tests for the Array package

use super::build_array_package;
use crate::{
    api::{CompileOptionsOverride, Engine, EngineOptions, Error},
    stdlib::{build_math_package, build_string_package},
    types::manager::TypeManager,
    values::{binder::Binder, dynamic::{RecordBuilder, Value}},
};
use bumpalo::Bump;

#[test]
fn test_array_package_builds() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let array = build_array_package(&arena, type_mgr, RecordBuilder::new(&arena, type_mgr)).build().unwrap();
    let record = array.as_record().unwrap();

    // Should have all functions
    assert!(!record.is_empty());
    assert!(record.get("Len").is_some());
    assert!(record.get("IsEmpty").is_some());
    assert!(record.get("Slice").is_some());
    assert!(record.get("Concat").is_some());
    assert!(record.get("Flatten").is_some());
    assert!(record.get("Zip").is_some());
    assert!(record.get("Reverse").is_some());
    assert!(record.get("Map").is_some());
}

/// Evaluates a Melbi expression with all standard packages (Array, Math, String).
fn eval<'a>(arena: &'a Bump, source: &'a str) -> Result<Value<'a, 'a>, Error> {
    let options = EngineOptions::default();

    let engine = Engine::new(options, arena, |arena, type_mgr, env| {
        let array = build_array_package(arena, type_mgr, RecordBuilder::new(arena, type_mgr)).build().unwrap();
        let env = env.bind("Array", array);
        let math = build_math_package(arena, type_mgr, RecordBuilder::new(arena, type_mgr)).build().unwrap();
        let env = env.bind("Math", math);
        let string = build_string_package(arena, type_mgr, RecordBuilder::new(arena, type_mgr)).build().unwrap();
        env.bind("String", string)
    });

    let compile_opts = CompileOptionsOverride::default();
    let expr = engine.compile(compile_opts, source, &[])?;
    expr.run(Default::default(), arena, &[])
}

// ============================================================================
// Len Tests
// ============================================================================

#[test]
fn test_len() {
    let arena = Bump::new();

    // Different types - polymorphism
    assert!(
        eval(&arena, "Array.Len([1, 2, 3, 4, 5]) == 5")
            .unwrap()
            .as_bool()
            .unwrap()
    );
    assert!(
        eval(&arena, "Array.Len([\"hello\", \"world\"]) == 2")
            .unwrap()
            .as_bool()
            .unwrap()
    );
    assert!(
        eval(&arena, "Array.Len([1.5, 2.5, 3.5]) == 3")
            .unwrap()
            .as_bool()
            .unwrap()
    );
    assert!(
        eval(&arena, "Array.Len([true, false, true]) == 3")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Empty array
    assert!(
        eval(&arena, "Array.Len([]) == 0")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Nested arrays
    assert!(
        eval(&arena, "Array.Len([[1,2], [3,4]]) == 2")
            .unwrap()
            .as_bool()
            .unwrap()
    );
}

// ============================================================================
// IsEmpty Tests
// ============================================================================

#[test]
fn test_is_empty() {
    let arena = Bump::new();

    assert!(
        eval(&arena, "Array.IsEmpty([]) == true")
            .unwrap()
            .as_bool()
            .unwrap()
    );
    assert!(
        eval(&arena, "Array.IsEmpty([1]) == false")
            .unwrap()
            .as_bool()
            .unwrap()
    );
    assert!(
        eval(&arena, "Array.IsEmpty([1, 2, 3]) == false")
            .unwrap()
            .as_bool()
            .unwrap()
    );
}

// ============================================================================
// Reverse Tests
// ============================================================================

#[test]
fn test_reverse() {
    let arena = Bump::new();

    // Basic reverse with integers
    assert!(
        eval(&arena, "Array.Reverse([1, 2, 3]) == [3, 2, 1]")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Reverse with strings
    assert!(
        eval(
            &arena,
            "Array.Reverse([\"a\", \"b\", \"c\"]) == [\"c\", \"b\", \"a\"]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );

    // Empty array
    assert!(
        eval(&arena, "Array.Reverse([]) == []")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Single element
    assert!(
        eval(&arena, "Array.Reverse([42]) == [42]")
            .unwrap()
            .as_bool()
            .unwrap()
    );
}

// ============================================================================
// Map Tests
// ============================================================================

#[test]
fn test_map() {
    let arena = Bump::new();

    // Basic map with integers - double each element
    assert!(
        eval(&arena, "Array.Map([1, 2, 3], (x) => x * 2) == [2, 4, 6]")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Map to different type - Int to Bool
    assert!(
        eval(
            &arena,
            "Array.Map([1, 2, 3], (x) => x > 1) == [false, true, true]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );

    // Single element
    assert!(
        eval(&arena, "Array.Map([42], (x) => x + 1) == [43]")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Complex expression in mapper
    assert!(
        eval(
            &arena,
            "Array.Map([1, 2, 3], (x) => x * x + 1) == [2, 5, 10]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );

    // Nested operations
    assert!(
        eval(
            &arena,
            "Array.Map([1, 2, 3], (x) => x * 2 + x) == [3, 6, 9]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );
}

#[test]
fn test_map_with_string_package() {
    let arena = Bump::new();

    // Map strings to their lengths
    assert!(
        eval(
            &arena,
            "Array.Map([\"a\", \"bb\", \"ccc\"], (s) => String.Len(s)) == [1, 2, 3]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );

    // Map strings to uppercase
    assert!(
        eval(
            &arena,
            "Array.Map([\"hello\", \"world\"], (s) => String.Upper(s)) == [\"HELLO\", \"WORLD\"]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );
}

#[test]
fn test_map_composition() {
    let arena = Bump::new();

    // Map then Reverse
    assert!(
        eval(
            &arena,
            "Array.Reverse(Array.Map([1, 2, 3], (x) => x * 2)) == [6, 4, 2]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );

    // Map then Len
    assert!(
        eval(
            &arena,
            "Array.Len(Array.Map([1, 2, 3, 4], (x) => x * 2)) == 4"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );

    // Map then Slice
    assert!(
        eval(
            &arena,
            "Array.Slice(Array.Map([1, 2, 3, 4, 5], (x) => x * 10), 1, 4) == [20, 30, 40]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );

    // Flatten then Map
    assert!(
        eval(
            &arena,
            "Array.Map(Array.Flatten([[1, 2], [3]]), (x) => x * 2) == [2, 4, 6]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );
}

#[test]
#[ignore = "TODO: Bug - empty arrays with different type variables don't compare equal"]
fn test_map_empty_array() {
    let arena = Bump::new();
    assert!(
        eval(&arena, "Array.Map([], (x) => x * 2) == []")
            .unwrap()
            .as_bool()
            .unwrap()
    );
}

#[test]
fn test_map_type_errors() {
    let arena = Bump::new();

    // Map expects function as second argument
    assert!(
        format!("{:?}", eval(&arena, "Array.Map([1, 2, 3], 42).err()")).contains("Type mismatch")
    );

    // Map expects array as first argument
    assert!(
        format!(
            "{:?}",
            eval(&arena, "Array.Map(\"not array\", (x) => x)").err()
        )
        .contains("Type mismatch")
    );

    // Function parameter type must match array element type
    assert!(
        format!(
            "{:?}",
            eval(&arena, "Array.Map([1, 2, 3], (s) => String.Len(s))").err()
        )
        .contains("Type mismatch")
    );
}

#[test]
#[ignore = "TODO: Bug - runtime errors are incorrectly wrapped as compilation errors during constant folding"]
fn test_map_runtime_error_propagation() {
    let arena = Bump::new();

    // Test that runtime errors from lambdas propagate correctly through Array.Map
    // Array.Map([0, 1, 6], (x) => [10, 20, 30][2 - x]) should fail at runtime
    // when x=6 because 2-6=-4 is out of bounds for a 3-element array.
    let result = eval(&arena, "Array.Map([0, 1, 6], (x) => [10, 20, 30][2 - x])");
    assert!(format!("{:?}", result.err()).contains("IndexOutOfBounds"));
}

// ============================================================================
// Slice Tests
// ============================================================================

#[test]
fn test_slice() {
    let arena = Bump::new();

    // Basic slice
    assert!(
        eval(&arena, "Array.Slice([1, 2, 3, 4, 5], 1, 4) == [2, 3, 4]")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Full array
    assert!(
        eval(&arena, "Array.Slice([1, 2, 3], 0, 3) == [1, 2, 3]")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Empty range (start == end)
    assert!(
        eval(&arena, "Array.Slice([1, 2, 3], 2, 2) == []")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Start > end
    assert!(
        eval(&arena, "Array.Slice([1, 2, 3], 3, 1) == []")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Start beyond length
    assert!(
        eval(&arena, "Array.Slice([1, 2, 3], 10, 20) == []")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // End beyond length (clamped)
    assert!(
        eval(&arena, "Array.Slice([1, 2, 3], 1, 100) == [2, 3]")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Empty array
    assert!(
        eval(&arena, "Array.Slice([], 0, 5) == []")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Slice of strings
    assert!(
        eval(
            &arena,
            "Array.Slice([\"a\", \"b\", \"c\", \"d\"], 1, 3) == [\"b\", \"c\"]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );
}

// ============================================================================
// Concat Tests
// ============================================================================

#[test]
fn test_concat() {
    let arena = Bump::new();

    // Basic concat
    assert!(
        eval(&arena, "Array.Concat([1, 2], [3, 4]) == [1, 2, 3, 4]")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Empty first array
    assert!(
        eval(&arena, "Array.Concat([], [1, 2]) == [1, 2]")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Empty second array
    assert!(
        eval(&arena, "Array.Concat([1, 2], []) == [1, 2]")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Both empty
    assert!(
        eval(&arena, "Array.Concat([], []) == []")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Strings (polymorphism)
    assert!(
        eval(
            &arena,
            "Array.Concat([\"a\", \"b\"], [\"c\", \"d\"]) == [\"a\", \"b\", \"c\", \"d\"]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );
}

// ============================================================================
// Flatten Tests
// ============================================================================

#[test]
fn test_flatten() {
    let arena = Bump::new();

    // Basic flatten
    assert!(
        eval(
            &arena,
            "Array.Flatten([[1, 2], [3, 4], [5]]) == [1, 2, 3, 4, 5]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );

    // With empty inner arrays
    assert!(
        eval(&arena, "Array.Flatten([[1], [], [2, 3]]) == [1, 2, 3]")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // All empty inner
    assert!(
        eval(&arena, "Array.Flatten([[], [], []]) == []")
            .unwrap()
            .as_bool()
            .unwrap()
    );
}

#[test]
#[ignore = "TODO: Bug - empty arrays with different type variables don't compare equal (Array.Flatten([]) returns Array[_N], [] is Array[_M])"]
fn test_flatten_empty_outer() {
    let arena = Bump::new();

    assert!(
        eval(&arena, "Array.Flatten([]) == []")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Strings
    assert!(
        eval(
            &arena,
            "Array.Flatten([[\"a\", \"b\"], [\"c\"]]) == [\"a\", \"b\", \"c\"]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );
}

// ============================================================================
// Zip Tests
// ============================================================================

#[test]
fn test_zip() {
    let arena = Bump::new();

    // Basic zip
    assert!(eval(&arena, "Array.Zip([1, 2, 3], [4, 5, 6]) == [{first = 1, second = 4}, {first = 2, second = 5}, {first = 3, second = 6}]").unwrap().as_bool().unwrap());

    // Different lengths - first shorter
    assert!(
        eval(
            &arena,
            "Array.Zip([1, 2], [3, 4, 5, 6]) == [{first = 1, second = 3}, {first = 2, second = 4}]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );

    // Different lengths - second shorter
    assert!(
        eval(
            &arena,
            "Array.Zip([1, 2, 3, 4], [5, 6]) == [{first = 1, second = 5}, {first = 2, second = 6}]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );

    // Different types
    assert!(eval(&arena, "Array.Zip([1, 2], [\"a\", \"b\"]) == [{first = 1, second = \"a\"}, {first = 2, second = \"b\"}]").unwrap().as_bool().unwrap());

    // Accessing tuple fields
    assert!(
        eval(&arena, "Array.Zip([1, 2], [3, 4])[0].first == 1")
            .unwrap()
            .as_bool()
            .unwrap()
    );
    assert!(
        eval(&arena, "Array.Zip([1, 2], [3, 4])[1].second == 4")
            .unwrap()
            .as_bool()
            .unwrap()
    );
}

#[test]
#[ignore = "TODO: Bug - empty arrays with different type variables don't compare equal"]
fn test_zip_both_empty() {
    let arena = Bump::new();
    assert!(
        eval(&arena, "Array.Zip([], []) == []")
            .unwrap()
            .as_bool()
            .unwrap()
    );
}

#[test]
#[ignore = "TODO: Bug - empty arrays with different type variables don't compare equal"]
fn test_zip_first_empty() {
    let arena = Bump::new();
    assert!(
        eval(&arena, "Array.Zip([], [1, 2, 3]) == []")
            .unwrap()
            .as_bool()
            .unwrap()
    );
}

#[test]
#[ignore = "TODO: Bug - empty arrays with different type variables don't compare equal"]
fn test_zip_second_empty() {
    let arena = Bump::new();
    assert!(
        eval(&arena, "Array.Zip([1, 2, 3], []) == []")
            .unwrap()
            .as_bool()
            .unwrap()
    );
}

// ============================================================================
// Composition and Chaining Tests
// ============================================================================

#[test]
fn test_composition() {
    let arena = Bump::new();

    // Reverse after Concat
    assert!(
        eval(
            &arena,
            "Array.Reverse(Array.Concat([1, 2], [3, 4])) == [4, 3, 2, 1]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );

    // Len of Slice
    assert!(
        eval(&arena, "Array.Len(Array.Slice([1, 2, 3, 4, 5], 1, 4)) == 3")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Reverse after Flatten
    assert!(
        eval(
            &arena,
            "Array.Reverse(Array.Flatten([[1, 2], [3]])) == [3, 2, 1]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );

    // Slice after Concat
    assert!(
        eval(
            &arena,
            "Array.Slice(Array.Concat([1, 2, 3], [4, 5, 6]), 2, 5) == [3, 4, 5]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );

    // Complex chain: Flatten, Reverse, Slice
    assert!(
        eval(
            &arena,
            "Array.Slice(Array.Reverse(Array.Flatten([[1, 2], [3, 4]])), 1, 3) == [3, 2]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );
}

// ============================================================================
// Type Safety Tests - Should fail at compile time
// ============================================================================

#[test]
fn test_type_errors() {
    let arena = Bump::new();

    // Len expects array, not string
    assert!(
        format!("{:?}", eval(&arena, "Array.Len(\"not an array\")").err())
            .contains("Type mismatch")
    );

    // Concat expects same element types
    assert!(
        format!(
            "{:?}",
            eval(&arena, "Array.Concat([1, 2], [\"a\", \"b\"])").err()
        )
        .contains("Type mismatch")
    );

    // Slice expects Int indices, not strings
    assert!(
        format!(
            "{:?}",
            eval(&arena, "Array.Slice([1, 2, 3], \"start\", 2)").err()
        )
        .contains("Type mismatch")
    );

    // Slice expects Int indices, not floats
    assert!(
        format!("{:?}", eval(&arena, "Array.Slice([1, 2, 3], 1.5, 2)").err())
            .contains("Type mismatch")
    );

    // Flatten expects array of arrays
    assert!(
        format!("{:?}", eval(&arena, "Array.Flatten([1, 2, 3])").err()).contains("Type mismatch")
    );

    // IsEmpty expects array
    assert!(format!("{:?}", eval(&arena, "Array.IsEmpty(5)").err()).contains("Type mismatch"));

    // Reverse expects array
    assert!(
        format!(
            "{:?}",
            eval(&arena, "Array.Reverse(\"not an array\")").err()
        )
        .contains("Type mismatch")
    );
}

// ============================================================================
// Integration Tests with String and Math packages
// ============================================================================

#[test]
fn test_integration_with_string() {
    let arena = Bump::new();

    // Len of Split result
    assert!(
        eval(&arena, "Array.Len(String.Split(\"a,b,c\", \",\")) == 3")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Reverse Split result
    assert!(
        eval(
            &arena,
            "Array.Reverse(String.Split(\"a,b,c\", \",\")) == [\"c\", \"b\", \"a\"]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );

    // Concat two Split results
    assert!(eval(&arena, "Array.Concat(String.Split(\"a,b\", \",\"), String.Split(\"c,d\", \",\")) == [\"a\", \"b\", \"c\", \"d\"]").unwrap().as_bool().unwrap());

    // Slice of Split
    assert!(
        eval(
            &arena,
            "Array.Slice(String.Split(\"a,b,c,d\", \",\"), 1, 3) == [\"b\", \"c\"]"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );
}

#[test]
fn test_integration_with_math() {
    let arena = Bump::new();

    // Array of Math results
    assert!(
        eval(&arena, "Array.Len([Math.Floor(3.7), Math.Ceil(2.1)]) == 2")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Reverse array of floats with Math.PI
    assert!(
        eval(&arena, "Array.Reverse([Math.PI, Math.E])[0] == Math.E")
            .unwrap()
            .as_bool()
            .unwrap()
    );

    // Zip with Math results
    assert!(
        eval(
            &arena,
            "Array.Zip([1, 2], [Math.Floor(Math.PI), Math.Ceil(Math.E)])[0].second == 3"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );
}

#[test]
fn test_integration_combined() {
    let arena = Bump::new();

    // Complex: Split, Len, with Math
    assert!(
        eval(
            &arena,
            "Array.Len(String.Split(\"hello,world,test\", \",\")) == Math.Floor(Math.PI)"
        )
        .unwrap()
        .as_bool()
        .unwrap()
    );

    // Zip String results with Math results
    assert!(eval(&arena, "Array.Len(Array.Zip(String.Split(\"a,b\", \",\"), [Math.Floor(1.5), Math.Ceil(2.5)])) == 2").unwrap().as_bool().unwrap());
}
