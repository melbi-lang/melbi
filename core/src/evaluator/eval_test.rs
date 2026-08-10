//! Unit tests for the evaluator.
use bumpalo::Bump;

use super::*;
use crate::analyzer;
use crate::evaluator::eval::Evaluator;
use crate::evaluator::{EvaluatorOptions, ExecutionErrorKind, ResourceExceededError, RuntimeError};
use crate::parser::{self, Span};
use crate::types::manager::TypeManager;
use crate::values::dynamic::Value;
use crate::values::function::{FfiContext, NativeFunction};

struct Runner<'a> {
    arena: &'a Bump,
    type_mgr: &'a TypeManager<'a>,
}

impl<'a> Runner<'a> {
    fn new(arena: &'a Bump) -> Self {
        Self {
            arena,
            type_mgr: TypeManager::new(arena),
        }
    }
    fn run<'i>(
        &self,
        input: &'i str,
        globals: &[(&'a str, Value<'a, 'a>)],
        arguments: &[(&'a str, Value<'a, 'a>)],
    ) -> Result<Value<'a, 'a>, ExecutionError> {
        let input = self.arena.alloc_str(input);

        // Derive analyzer global types from evaluator global values.
        let global_types: alloc::vec::Vec<(&str, &crate::types::Type)> = globals
            .iter()
            .map(|(name, value)| (*name, value.ty))
            .collect();

        // Derive analyzer argument types from evaluator argument values.
        let argument_types: alloc::vec::Vec<(&str, &crate::types::Type)> = arguments
            .iter()
            .map(|(name, value)| (*name, value.ty))
            .collect();

        let parsed = parser::parse(self.arena, input).expect("parsing failed");
        let typed = analyzer::analyze(
            self.type_mgr,
            self.arena,
            parsed,
            &global_types,
            &argument_types,
        )
        .expect("type checking failed");

        Evaluator::new(
            EvaluatorOptions::default(),
            self.arena,
            self.type_mgr,
            typed,
            globals,
            arguments,
        )
        .eval()
    }

    fn run_with_limits<'i>(
        &self,
        input: &'i str,
        globals: &[(&'a str, Value<'a, 'a>)],
        arguments: &[(&'a str, Value<'a, 'a>)],
        max_stack_depth: usize,
    ) -> Result<Value<'a, 'a>, ExecutionError> {
        let input = self.arena.alloc_str(input);

        // Derive analyzer global types from evaluator global values.
        let global_types: alloc::vec::Vec<(&str, &crate::types::Type)> = globals
            .iter()
            .map(|(name, value)| (*name, value.ty))
            .collect();

        // Derive analyzer argument types from evaluator argument values.
        let argument_types: alloc::vec::Vec<(&str, &crate::types::Type)> = arguments
            .iter()
            .map(|(name, value)| (*name, value.ty))
            .collect();

        let parsed = parser::parse(self.arena, input).expect("parsing failed");
        let typed = analyzer::analyze(
            self.type_mgr,
            self.arena,
            parsed,
            &global_types,
            &argument_types,
        )
        .expect("type checking failed");

        Evaluator::new(
            EvaluatorOptions {
                max_depth: max_stack_depth,
            },
            self.arena,
            self.type_mgr,
            typed,
            globals,
            arguments,
        )
        .eval()
    }
}

// ============================================================================
// Constants
// ============================================================================

#[test]
fn constant_int() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("42", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn constant_negative_int() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("-42", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), -42);
}

#[test]
fn constant_float() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("3.14", &[], &[]).unwrap();
    assert_eq!(result.as_float().unwrap(), 3.14);
}

#[test]
fn constant_bool_true() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("true", &[], &[]).unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn constant_bool_false() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("false", &[], &[]).unwrap();
    assert!(!result.as_bool().unwrap());
}

#[test]
fn constant_string() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run(r#""hello""#, &[], &[]).unwrap();
    assert_eq!(result.as_str().unwrap(), "hello");
}

// ============================================================================
// Integer Arithmetic
// ============================================================================

#[test]
fn int_addition() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("2 + 3", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn int_subtraction() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("10 - 4", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 6);
}

#[test]
fn int_multiplication() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("3 * 4", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 12);
}

#[test]
fn int_division() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("10 / 2", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn int_division_truncates() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("7 / 3", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn int_power() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("2 ^ 10", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 1024);
}

#[test]
fn int_power_zero() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("5 ^ 0", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 1);
}

#[test]
fn int_division_by_zero() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("10 / 0", &[], &[]);
    assert!(matches!(
        result,
        Err(ExecutionError {
            kind: ExecutionErrorKind::Runtime(RuntimeError::DivisionByZero {}),
            ..
        })
    ));
}

#[test]
fn int_division_euclidean_negative_dividend() {
    // Euclidean division: -7 / 3 = -3 (not -2 like truncated division)
    // because -7 = -3 * 3 + 2 (remainder is always non-negative)
    let arena = Bump::new();
    let result = Runner::new(&arena).run("-7 / 3", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), -3);
}

#[test]
fn int_division_euclidean_negative_divisor() {
    // Euclidean division: 7 / -3 = -2 (not -2 like truncated, same in this case)
    // because 7 = -2 * (-3) + 1
    let arena = Bump::new();
    let result = Runner::new(&arena).run("7 / -3", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), -2);
}

#[test]
fn int_division_euclidean_both_negative() {
    // Euclidean division: -7 / -3 = 3 (not 2 like truncated)
    // because -7 = 3 * (-3) + 2
    let arena = Bump::new();
    let result = Runner::new(&arena).run("-7 / -3", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 3);
}

#[test]
fn int_division_i64_min_overflow() {
    // i64::MIN / -1 would overflow (result would be i64::MAX + 1)
    let arena = Bump::new();
    let result = Runner::new(&arena).run("-9223372036854775808 / -1", &[], &[]);
    assert!(matches!(
        result,
        Err(ExecutionError {
            kind: ExecutionErrorKind::Runtime(RuntimeError::IntegerOverflow {}),
            ..
        })
    ));
}

#[test]
fn int_wrapping_overflow_add() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("9223372036854775807 + 1", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), i64::MIN);
}

#[test]
fn int_wrapping_overflow_mul() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("9223372036854775807 * 2", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), -2);
}

// ============================================================================
// Float Arithmetic
// ============================================================================

#[test]
fn float_addition() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("3.14 + 2.0", &[], &[]).unwrap();
    assert!((result.as_float().unwrap() - 5.14).abs() < 0.0001);
}

#[test]
fn float_subtraction() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("10.5 - 3.5", &[], &[]).unwrap();
    assert_eq!(result.as_float().unwrap(), 7.0);
}

#[test]
fn float_multiplication() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("2.5 * 4.0", &[], &[]).unwrap();
    assert_eq!(result.as_float().unwrap(), 10.0);
}

#[test]
fn float_division() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("10.0 / 3.0", &[], &[]).unwrap();
    let expected = 10.0 / 3.0;
    assert!((result.as_float().unwrap() - expected).abs() < 0.0001);
}

#[test]
fn float_division_by_zero() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("10.0 / 0.0", &[], &[]).unwrap();
    assert!(result.as_float().unwrap().is_infinite());
}

#[test]
fn float_power() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("2.0 ^ 3.0", &[], &[]).unwrap();
    assert_eq!(result.as_float().unwrap(), 8.0);
}

// ============================================================================
// Boolean Operators (Milestone 1.3) - Short-Circuit Evaluation
// ============================================================================

#[test]
fn boolean_and_true_true() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("true and true", &[], &[]).unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn boolean_and_true_false() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("true and false", &[], &[]).unwrap();
    assert!(!result.as_bool().unwrap());
}

#[test]
fn boolean_and_false_true() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("false and true", &[], &[]).unwrap();
    assert!(!result.as_bool().unwrap());
}

#[test]
fn boolean_and_false_false() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("false and false", &[], &[])
        .unwrap();
    assert!(!result.as_bool().unwrap());
}

#[test]
fn boolean_or_true_true() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("true or true", &[], &[]).unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn boolean_or_true_false() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("true or false", &[], &[]).unwrap();
    // Right side not evaluated due to short-circuit
    assert!(result.as_bool().unwrap());
}

#[test]
fn boolean_or_false_true() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("false or true", &[], &[]).unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn boolean_or_false_false() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("false or false", &[], &[]).unwrap();
    assert!(!result.as_bool().unwrap());
}

#[test]
fn boolean_chain_and() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("true and true and false", &[], &[])
        .unwrap();
    assert!(!result.as_bool().unwrap());
}

#[test]
fn boolean_chain_or() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("false or false or true", &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn boolean_mixed_chain() {
    let arena = Bump::new();
    // 'and' has higher precedence than 'or'
    // So: true and false or true = (true and false) or true = false or true = true
    let result = Runner::new(&arena)
        .run("true and false or true", &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn boolean_with_variables() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [
        ("x", Value::bool(runner.type_mgr, true)),
        ("y", Value::bool(runner.type_mgr, false)),
    ];
    let result = runner.run("x and y", &[], &var_values).unwrap();
    assert!(!result.as_bool().unwrap());
}

#[test]
fn boolean_short_circuit_and_with_where() {
    let arena = Bump::new();
    // false and (x where { x = true })
    // The where expression should not be evaluated due to short-circuit
    let result = Runner::new(&arena)
        .run("false and (x where { x = true })", &[], &[])
        .unwrap();
    assert!(!result.as_bool().unwrap());
}

#[test]
fn boolean_short_circuit_or_with_where() {
    let arena = Bump::new();
    // true or (x where { x = false })
    // The where expression should not be evaluated due to short-circuit
    let result = Runner::new(&arena)
        .run("true or (x where { x = false })", &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

// ============================================================================
// Nested Expressions
// ============================================================================

#[test]
fn nested_arithmetic() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("(2 + 3) * 4", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 20);
}

#[test]
fn deeply_nested() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("((1 + 2) * (3 + 4)) - (5 * 6)", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), (1 + 2) * (3 + 4) - (5 * 6));
}

#[test]
fn operator_precedence() {
    let arena = Bump::new();

    // Verify that * binds tighter than +
    let result = Runner::new(&arena).run("2 + 3 * 4", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 14); // Not 20

    // Verify that ^ binds tighter than *
    let result = Runner::new(&arena).run("2 * 3 ^ 2", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 18); // Not 36
}

// ============================================================================
// Stack Depth Limit
// ============================================================================

#[test]
#[ignore = "TODO(investigage): this is actually overflowing the stack"]
fn stack_depth_limit() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    // Create a deeply nested expression using actual operations: 1 + (1 + (1 + ...))
    // This creates real recursion depth, unlike just parentheses
    let mut source = String::from("1");
    for _ in 0..100 {
        source = format!("1 + ({source})");
    }

    // With default limit of 1000, this should succeed (100 < 1000)
    let result = runner.run(&source, &[], &[]);
    assert!(result.is_ok());

    // But with a lower limit of 50, it should fail
    let result = runner.run_with_limits(&source, &[], &[], 50);
    assert!(matches!(
        result,
        Err(ExecutionError {
            kind: ExecutionErrorKind::ResourceExceeded(ResourceExceededError::StackOverflow { .. }),
            ..
        })
    ));
}

#[test]
fn custom_stack_depth_limit() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Create expression within custom limit
    let mut source = String::from("1");
    for _ in 0..50 {
        source = format!("({source} + 1)");
    }

    let parsed = parser::parse(&arena, &source).expect("Parse failed");
    let typed =
        analyzer::analyze(type_manager, &arena, parsed, &[], &[]).expect("Type-check failed");

    // With custom limit of 100, this should succeed
    let result = Evaluator::new(
        EvaluatorOptions { max_depth: 100 },
        &arena,
        type_manager,
        typed,
        &[],
        &[],
    )
    .eval();
    assert!(result.is_ok());

    // But with limit of 40, it should fail
    let result = Evaluator::new(
        EvaluatorOptions { max_depth: 40 },
        &arena,
        type_manager,
        typed,
        &[],
        &[],
    )
    .eval();
    assert!(matches!(
        result,
        Err(ExecutionError {
            kind: ExecutionErrorKind::ResourceExceeded(ResourceExceededError::StackOverflow { .. }),
            ..
        })
    ));
}

// ============================================================================
// Variables (Runtime Parameters)
// ============================================================================

#[test]
fn variable_simple_lookup() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [("x", Value::int(runner.type_mgr, 42))];
    let result = runner.run("x", &[], &var_values).unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn variable_in_expression() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [("x", Value::int(runner.type_mgr, 5))];
    let result = runner.run("x * 2 + 10", &[], &var_values).unwrap();
    assert_eq!(result.as_int().unwrap(), 20); // 5 * 2 + 10 = 20
}

#[test]
fn multiple_variables() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [
        ("x", Value::int(runner.type_mgr, 10)),
        ("y", Value::int(runner.type_mgr, 20)),
    ];
    let result = runner.run("x + y * 2", &[], &var_values).unwrap();
    assert_eq!(result.as_int().unwrap(), 50); // 10 + 20 * 2 = 50
}

#[test]
fn variable_float() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [("price", Value::float(runner.type_mgr, 100.0))];
    let result = runner.run("price * 1.2", &[], &var_values).unwrap();
    assert_eq!(result.as_float().unwrap(), 120.0);
}

#[test]
fn variable_bool() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [("flag", Value::bool(runner.type_mgr, true))];
    let result = runner.run("flag", &[], &var_values).unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn variable_string() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [("name", Value::str(&arena, runner.type_mgr.str(), "Alice"))];
    let result = runner.run("name", &[], &var_values).unwrap();
    assert_eq!(result.as_str().unwrap(), "Alice");
}

// ============================================================================
// Globals (Constants and Built-in Functions)
// ============================================================================

#[test]
fn global_constant_pi() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let globals_values = [("PI", Value::float(runner.type_mgr, 3.14159))];
    let result = runner.run("PI * 2.0", &globals_values, &[]).unwrap();
    assert!((result.as_float().unwrap() - 6.28318).abs() < 0.0001);
}

#[test]
fn global_constant_with_variables() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let globals_values = [("PI", Value::float(runner.type_mgr, 3.14159))];
    let var_values = [("radius", Value::float(runner.type_mgr, 5.0))];
    let result = runner
        .run("PI * radius * radius", &globals_values, &var_values)
        .unwrap();

    // Area = PI * r^2 = 3.14159 * 5 * 5 = 78.53975
    assert!((result.as_float().unwrap() - 78.53975).abs() < 0.001);
}

#[test]
fn multiple_globals() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let globals_values = [
        ("E", Value::float(runner.type_mgr, 2.71828)),
        ("PI", Value::float(runner.type_mgr, 3.14159)),
    ];
    let result = runner.run("PI + E", &globals_values, &[]).unwrap();
    assert!((result.as_float().unwrap() - 5.85987).abs() < 0.0001);
}

// ============================================================================
// Shadowing Tests (Variables vs Where Bindings)
// ============================================================================

#[test]
fn where_shadows_variable() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [("x", Value::int(runner.type_mgr, 10))];
    let result = runner.run("x where { x = 5 }", &[], &var_values).unwrap();

    // Inner x = 5 shadows outer x = 10
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn where_can_reference_variable() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [("x", Value::int(runner.type_mgr, 10))];
    let result = runner
        .run("y where { y = x * 2 }", &[], &var_values)
        .unwrap();

    // y = x * 2 = 10 * 2 = 20
    assert_eq!(result.as_int().unwrap(), 20);
}

// ============================================================================
// Where Expressions (Local Scoping)
// ============================================================================

#[test]
fn where_simple() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("x where { x = 42 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn where_multiple_bindings() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("x + y where { x = 10, y = 20 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 30);
}

#[test]
fn where_sequential_binding() {
    let arena = Bump::new();
    // b can reference a (sequential binding)
    let result = Runner::new(&arena)
        .run("b where { a = 1, b = a + 1 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn where_sequential_binding_chain() {
    let arena = Bump::new();
    // c can reference b which references a
    let result = Runner::new(&arena)
        .run("c where { a = 1, b = a * 2, c = b + 1 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 3); // a=1, b=2, c=3
}

#[test]
fn where_complex_expression() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("a + b + c where { a = 1, b = a * 2, c = b + 1 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 6); // 1 + 2 + 3 = 6
}

#[test]
fn where_nested_scopes() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("x + y where { x = 10 } where { y = 20 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 30);
}

#[test]
fn where_with_arithmetic() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("(a + b) * c where { a = 2, b = 3, c = 4 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 20); // (2 + 3) * 4 = 20
}

#[test]
fn where_with_float() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("x * y where { x = 2.5, y = 4.0 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_float().unwrap(), 10.0);
}

// ============================================================================
// Records (Milestone 2.2)
// ============================================================================

#[test]
fn record_empty() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("Record{}", &[], &[]).unwrap();
    let record = result.as_record().unwrap();
    assert_eq!(record.len(), 0);
    assert_eq!(format!("{result}"), "{}");
}

#[test]
fn record_simple() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("{ x = 42, y = 3.14 }", &[], &[])
        .unwrap();
    let record = result.as_record().unwrap();
    assert_eq!(record.len(), 2);

    let x = record.get("x").unwrap();
    assert_eq!(x.as_int().unwrap(), 42);

    let y = record.get("y").unwrap();
    assert!((y.as_float().unwrap() - 3.14).abs() < 0.0001);
}

#[test]
fn field_access_simple() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("{ x = 42 }.x", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn field_access_multiple_fields() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("{ a = 10, b = 20, c = 30 }.b", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 20);
}

#[test]
fn field_access_in_expression() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("{ x = 5, y = 10 }.x + { x = 5, y = 10 }.y", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 15);
}

#[test]
fn record_with_where() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("{ x = a, y = b } where { a = 1, b = 2 }", &[], &[])
        .unwrap();
    let record = result.as_record().unwrap();

    let x = record.get("x").unwrap();
    assert_eq!(x.as_int().unwrap(), 1);

    let y = record.get("y").unwrap();
    assert_eq!(y.as_int().unwrap(), 2);
}

#[test]
fn nested_record() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            "{ point = { x = 10, y = 20 }, name = \"origin\" }",
            &[],
            &[],
        )
        .unwrap();
    let outer = result.as_record().unwrap();

    let name = outer.get("name").unwrap();
    assert_eq!(name.as_str().unwrap(), "origin");

    let point = outer.get("point").unwrap();
    let point_rec = point.as_record().unwrap();

    let x = point_rec.get("x").unwrap();
    assert_eq!(x.as_int().unwrap(), 10);

    let y = point_rec.get("y").unwrap();
    assert_eq!(y.as_int().unwrap(), 20);
}

#[test]
fn nested_field_access() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("{ point = { x = 10, y = 20 } }.point.x", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 10);
}

#[test]
fn math_package_record() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    // Create Math record type with PI and E fields
    let math_ty = runner.type_mgr.record(&[
        ("E", runner.type_mgr.float()),
        ("PI", runner.type_mgr.float()),
    ]);

    // Create Math record value with PI and E
    let math_value = Value::record(
        &arena,
        math_ty,
        &[
            ("E", Value::float(runner.type_mgr, 2.71828)),
            ("PI", Value::float(runner.type_mgr, 3.14159)),
        ],
    )
    .unwrap();

    let globals_values = [("Math", math_value)];
    let result = runner
        .run("Math.PI * 2.0 + Math.E", &globals_values, &[])
        .unwrap();

    // Math.PI * 2.0 + Math.E = 3.14159 * 2.0 + 2.71828 = 6.28318 + 2.71828 = 9.00146
    assert!((result.as_float().unwrap() - 9.00146).abs() < 0.0001);
}

#[test]
fn math_package_circle_area() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    // Create Math record type
    let math_ty = runner.type_mgr.record(&[("PI", runner.type_mgr.float())]);

    // Create Math record value
    let math_value = Value::record(
        &arena,
        math_ty,
        &[("PI", Value::float(runner.type_mgr, 3.14159))],
    )
    .unwrap();

    let globals_values = [("Math", math_value)];
    let var_values = [("radius", Value::float(runner.type_mgr, 5.0))];
    let result = runner
        .run("Math.PI * radius * radius", &globals_values, &var_values)
        .unwrap();

    // Area = Math.PI * r^2 = 3.14159 * 5 * 5 = 78.53975
    assert!((result.as_float().unwrap() - 78.53975).abs() < 0.001);
}

// ================================
// Unary Operator Tests
// ================================

#[test]
fn unary_negation_int() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("-42", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), -42);
}

#[test]
fn unary_negation_int_positive() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("-(42)", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), -42);
}

#[test]
fn unary_double_negation() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("-(-5)", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn unary_negation_expression() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("-(1 + 2)", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), -3);
}

#[test]
fn unary_negation_float() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("-(3.14)", &[], &[]).unwrap();
    assert!((result.as_float().unwrap() + 3.14).abs() < 0.0001);
}

#[test]
fn unary_negation_float_expression() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("-(2.5 + 1.5)", &[], &[]).unwrap();
    assert!((result.as_float().unwrap() + 4.0).abs() < 0.0001);
}

#[test]
fn unary_not_true() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("not true", &[], &[]).unwrap();
    assert!(!result.as_bool().unwrap());
}

#[test]
fn unary_not_false() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("not false", &[], &[]).unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn unary_not_expression() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("not (true and false)", &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn unary_with_where() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("-x where { x = 42 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), -42);
}

#[test]
fn unary_negation_wrapping() {
    let arena = Bump::new();
    // Use string interpolation to build the source with i64::MIN
    let source = format!("-({})", i64::MIN);
    let result = Runner::new(&arena).run(&source, &[], &[]).unwrap();
    // -i64::MIN wraps to i64::MIN
    assert_eq!(result.as_int().unwrap(), i64::MIN);
}

// ================================
// If/Else Expression Tests
// ================================

#[test]
fn if_true_branch() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("if true then 1 else 2", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 1);
}

#[test]
fn if_false_branch() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("if false then 1 else 2", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn if_with_variable() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    // Test with flag = true
    let var_values = [("flag", Value::bool(runner.type_mgr, true))];
    let result = runner
        .run("if flag then 10 else 20", &[], &var_values)
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 10);

    // Test with flag = false
    let var_values = [("flag", Value::bool(runner.type_mgr, false))];
    let result = runner
        .run("if flag then 10 else 20", &[], &var_values)
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 20);
}

#[test]
fn if_with_expression_condition() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("if true and false then 1 else 2", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn if_with_where() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("if x then 1 else 2 where { x = true }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 1);
}

#[test]
fn if_nested() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("if true then (if false then 1 else 2) else 3", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn if_float_branches() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("if true then 3.14 else 2.71", &[], &[])
        .unwrap();
    assert!((result.as_float().unwrap() - 3.14).abs() < 0.0001);
}

#[test]
fn if_string_branches() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"if false then "yes" else "no""#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "no");
}

#[test]
fn if_bool_branches() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("if true then true else false", &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn if_with_complex_expressions() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("if true then (1 + 2) * 3 else 4 ^ 2", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 9);
}

// ================================
// Array Tests
// ================================

#[test]
fn array_empty() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("[]", &[], &[]).unwrap();

    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 0);
}

#[test]
fn array_simple_int() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("[1, 2, 3]", &[], &[]).unwrap();

    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).unwrap().as_int().unwrap(), 1);
    assert_eq!(array.get(1).unwrap().as_int().unwrap(), 2);
    assert_eq!(array.get(2).unwrap().as_int().unwrap(), 3);
}

#[test]
fn array_simple_float() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[3.14, 2.71, 1.41]", &[], &[])
        .unwrap();

    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert!((array.get(0).unwrap().as_float().unwrap() - 3.14).abs() < 0.001);
    assert!((array.get(1).unwrap().as_float().unwrap() - 2.71).abs() < 0.001);
    assert!((array.get(2).unwrap().as_float().unwrap() - 1.41).abs() < 0.001);
}

#[test]
fn array_simple_bool() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[true, false, true]", &[], &[])
        .unwrap();

    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert!(array.get(0).unwrap().as_bool().unwrap());
    assert!(!array.get(1).unwrap().as_bool().unwrap());
    assert!(array.get(2).unwrap().as_bool().unwrap());
}

#[test]
fn array_simple_string() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"["a", "b", "c"]"#, &[], &[])
        .unwrap();

    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).unwrap().as_str().unwrap(), "a");
    assert_eq!(array.get(1).unwrap().as_str().unwrap(), "b");
    assert_eq!(array.get(2).unwrap().as_str().unwrap(), "c");
}

#[test]
fn array_with_expressions() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[1 + 1, 2 * 2, 3 ^ 2]", &[], &[])
        .unwrap();

    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).unwrap().as_int().unwrap(), 2);
    assert_eq!(array.get(1).unwrap().as_int().unwrap(), 4);
    assert_eq!(array.get(2).unwrap().as_int().unwrap(), 9);
}

#[test]
fn array_nested() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[[1, 2], [3, 4]]", &[], &[])
        .unwrap();

    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);

    let inner1 = array.get(0).unwrap().as_array().unwrap();
    assert_eq!(inner1.len(), 2);
    assert_eq!(inner1.get(0).unwrap().as_int().unwrap(), 1);
    assert_eq!(inner1.get(1).unwrap().as_int().unwrap(), 2);

    let inner2 = array.get(1).unwrap().as_array().unwrap();
    assert_eq!(inner2.len(), 2);
    assert_eq!(inner2.get(0).unwrap().as_int().unwrap(), 3);
    assert_eq!(inner2.get(1).unwrap().as_int().unwrap(), 4);
}

#[test]
fn array_with_where() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[x, y, z] where { x = 1, y = 2, z = 3 }", &[], &[])
        .unwrap();

    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).unwrap().as_int().unwrap(), 1);
    assert_eq!(array.get(1).unwrap().as_int().unwrap(), 2);
    assert_eq!(array.get(2).unwrap().as_int().unwrap(), 3);
}

#[test]
fn array_with_variables() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [
        ("x", Value::int(runner.type_mgr, 10)),
        ("y", Value::int(runner.type_mgr, 20)),
        ("z", Value::int(runner.type_mgr, 30)),
    ];
    let result = runner.run("[x, y, z]", &[], &var_values).unwrap();
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 3);
    assert_eq!(array.get(0).unwrap().as_int().unwrap(), 10);
    assert_eq!(array.get(1).unwrap().as_int().unwrap(), 20);
    assert_eq!(array.get(2).unwrap().as_int().unwrap(), 30);
}

// ================================
// Array Indexing Tests
// ================================

#[test]
fn index_simple() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("[1, 2, 3][0]", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 1);
}

#[test]
fn index_last_element() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("[1, 2, 3][2]", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 3);
}

#[test]
fn index_with_variable() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("arr[i] where { arr = [10, 20, 30], i = 1 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 20);
}

#[test]
fn index_with_expression() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[5, 10, 15][1 + 1]", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 15);
}

#[test]
fn index_out_of_bounds_positive() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("[1, 2][5]", &[], &[]);
    assert!(matches!(
        result,
        Err(ExecutionError {
            kind: ExecutionErrorKind::Runtime(RuntimeError::IndexOutOfBounds { index: 5, len: 2 }),
            ..
        })
    ));
}

#[test]
fn index_negative() {
    let arena = Bump::new();
    // -1 should get the last element
    let result = Runner::new(&arena).run("[1, 2][-1]", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn index_negative_second_to_last() {
    let arena = Bump::new();
    // -2 should get the second-to-last element
    let result = Runner::new(&arena).run("[1, 2, 3][-2]", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn index_negative_first() {
    let arena = Bump::new();
    // -3 should get the first element of a 3-element array
    let result = Runner::new(&arena)
        .run("[10, 20, 30][-3]", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 10);
}

#[test]
fn index_out_of_bounds_negative() {
    let arena = Bump::new();
    // -3 is out of bounds for a 2-element array
    let result = Runner::new(&arena).run("[1, 2][-3]", &[], &[]);
    assert!(result.is_err());
    assert_eq!(
        &result.unwrap_err().kind,
        &ExecutionErrorKind::Runtime(RuntimeError::IndexOutOfBounds { index: -3, len: 2 })
    );
}

#[test]
fn index_nested_array() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[[1, 2], [3, 4]][1][0]", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 3);
}

#[test]
fn index_float_array() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[3.14, 2.71, 1.41][1]", &[], &[])
        .unwrap();
    assert!((result.as_float().unwrap() - 2.71).abs() < 0.001);
}

#[test]
fn index_string_array() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"["a", "b", "c"][2]"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "c");
}

#[test]
fn index_bool_array() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[true, false, true][1]", &[], &[])
        .unwrap();
    assert!(!result.as_bool().unwrap());
}

#[test]
fn index_with_where_binding() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            "arr[idx] where { arr = [100, 200, 300], idx = 2 }",
            &[],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 300);
}

// ================================
// Map Indexing Tests
// ================================

#[test]
fn map_index_basic_int_key() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("{1: \"one\"}[1]", &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "one");
}

#[test]
fn map_index_key_not_found() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("{1: \"one\"}[0]", &[], &[]);
    assert!(matches!(
        result,
        Err(ExecutionError {
            kind: ExecutionErrorKind::Runtime(RuntimeError::KeyNotFound { .. }),
            ..
        })
    ));
}

#[test]
fn map_index_in_function() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("f({1: \"one\"}) where { f = (m) => m[1] }", &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "one");
}

#[test]
fn map_index_with_variable_key() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("f({1: \"one\"}, 1) where { f = (m, k) => m[k] }", &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "one");
}

#[test]
fn map_index_multiple_key_types() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            r#"[f({1: "one"}, 1), f({"one": "uno"}, "one")] where { f = (m, k) => m[k] }"#,
            &[],
            &[],
        )
        .unwrap();
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array.get(0).unwrap().as_str().unwrap(), "one");
    assert_eq!(array.get(1).unwrap().as_str().unwrap(), "uno");
}

#[test]
fn polymorphic_lambda_array_construction() {
    // Array construction in polymorphic lambda body with monomorphization
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r"f(10, 42) where { f = (a, b) => [b, a] }", &[], &[])
        .unwrap();
    let array = result.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array.get(0).unwrap().as_int().unwrap(), 42);
    assert_eq!(array.get(1).unwrap().as_int().unwrap(), 10);
}

#[test]
fn polymorphic_lambda_empty_record() {
    // Empty record/map construction needs to know the concrete types
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            r"f(10, 42) where { f = (a, b) => if false then {a: b} else {} }",
            &[],
            &[],
        )
        .unwrap();
    let map = result.as_map().unwrap();
    assert_eq!(map.len(), 0);
}

#[test]
fn polymorphic_lambda_empty_map_no_params() {
    // Empty map construction in polymorphic lambda body
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r"f() where { f = () => {} }", &[], &[])
        .unwrap();
    // {} can be either an empty map or empty record depending on context
    // In this case it should be a map since there's no type constraint
    if let Ok(map) = result.as_map() {
        assert_eq!(map.len(), 0);
    } else if let Ok(record) = result.as_record() {
        assert_eq!(record.len(), 0);
    } else {
        panic!("Expected map or record, got: {result:?}");
    }
}

#[test]
fn map_index_nested() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("{1: {2: \"value\"}}[1][2]", &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "value");
}

#[test]
fn map_index_string_keys() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"{"key": "value"}["key"]"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "value");
}

#[test]
fn map_index_bool_keys() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"{true: "yes", false: "no"}[true]"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "yes");
}

#[test]
fn map_index_empty_map() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("{}[1]", &[], &[]);
    assert!(matches!(
        result,
        Err(ExecutionError {
            kind: ExecutionErrorKind::Runtime(RuntimeError::KeyNotFound { .. }),
            ..
        })
    ));
}

#[test]
fn map_index_key_not_found_with_otherwise() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"{1: "one"}[0] otherwise "fallback""#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "fallback");
}

// ================================
// Format String Tests
// ================================

#[test]
fn format_str_no_interpolation() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"f"hello world""#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "hello world");
}

#[test]
fn format_str_single_int() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"f"x = {x}" where { x = 42 }"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "x = 42");
}

#[test]
fn format_str_multiple_values() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"f"{a} + {b} = {a + b}" where { a = 1, b = 2 }"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "1 + 2 = 3");
}

#[test]
fn format_str_with_string() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"f"Hello, {name}!" where { name = "World" }"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "Hello, World!");
}

#[test]
fn format_str_with_float() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"f"Pi = {pi}" where { pi = 3.14 }"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "Pi = 3.14");
}

#[test]
fn format_str_with_bool() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"f"Flag: {flag}" where { flag = true }"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "Flag: true");
}

#[test]
fn format_str_with_array() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"f"Array: {arr}" where { arr = [1, 2, 3] }"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "Array: [1, 2, 3]");
}

#[test]
fn format_str_consecutive_expressions() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"f"{x}{y}" where { x = 1, y = 2 }"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "12");
}

#[test]
fn format_str_mixed_types() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            r#"f"Int: {i}, Float: {f}, Bool: {b}" where { i = 42, f = 3.14, b = true }"#,
            &[],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "Int: 42, Float: 3.14, Bool: true");
}

#[test]
fn format_str_with_variables() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let var_values = [
        ("age", Value::int(runner.type_mgr, 30)),
        ("name", Value::str(&arena, runner.type_mgr.str(), "Alice")),
    ];
    let result = runner
        .run(r#"f"{name} is {age} years old""#, &[], &var_values)
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "Alice is 30 years old");
}

#[test]
fn format_str_string_no_quotes() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"f"Result: {s}" where { s = "test" }"#, &[], &[])
        .unwrap();
    // String should NOT have quotes in the output
    assert_eq!(result.as_str().unwrap(), "Result: test");
}

#[test]
fn format_str_array_with_strings() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            r#"f"Items: {items}" where { items = ["a", "b", "c"] }"#,
            &[],
            &[],
        )
        .unwrap();
    // Array uses Debug, so strings inside should have quotes
    assert_eq!(result.as_str().unwrap(), r#"Items: ["a", "b", "c"]"#);
}

// ================================
// Otherwise Operator Tests
// ================================

#[test]
fn otherwise_no_error() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("(10 / 2) otherwise -1", &[], &[])
        .unwrap();

    // Primary succeeds, return its value
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn otherwise_division_by_zero() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("(10 / 0) otherwise -1", &[], &[])
        .unwrap();

    // Primary fails (division by zero), return fallback
    assert_eq!(result.as_int().unwrap(), -1);
}

#[test]
fn otherwise_index_out_of_bounds() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[1, 2][5] otherwise -1", &[], &[])
        .unwrap();

    // Primary fails (index out of bounds), return fallback
    assert_eq!(result.as_int().unwrap(), -1);
}

#[test]
fn otherwise_negative_index() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[1, 2][-1] otherwise 99", &[], &[])
        .unwrap();
    // Negative indices now work, so -1 returns the last element
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn otherwise_negative_index_out_of_bounds() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("[1, 2][-3] otherwise 99", &[], &[])
        .unwrap();
    // -3 is out of bounds, so fallback to otherwise clause
    assert_eq!(result.as_int().unwrap(), 99);
}

#[test]
fn otherwise_with_variables() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    // Test with valid index
    let arr_value = Value::array(
        &arena,
        runner.type_mgr.array(runner.type_mgr.int()),
        &[
            Value::int(runner.type_mgr, 10),
            Value::int(runner.type_mgr, 20),
            Value::int(runner.type_mgr, 30),
        ],
    )
    .unwrap();
    let var_values = [
        ("arr", arr_value),
        ("default", Value::int(runner.type_mgr, -1)),
        ("idx", Value::int(runner.type_mgr, 1)),
    ];
    let result = runner
        .run("arr[idx] otherwise default", &[], &var_values)
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 20);

    // Test with invalid index
    let var_values = [
        ("arr", arr_value),
        ("default", Value::int(runner.type_mgr, -1)),
        ("idx", Value::int(runner.type_mgr, 10)),
    ];
    let result = runner
        .run("arr[idx] otherwise default", &[], &var_values)
        .unwrap();
    assert_eq!(result.as_int().unwrap(), -1);
}

#[test]
fn otherwise_nested() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("(10 / 0) otherwise ((5 / 0) otherwise 42)", &[], &[])
        .unwrap();
    // Both primary and first fallback fail, return nested fallback
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn otherwise_with_where() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            "(arr[i] otherwise def) where { arr = [1, 2], i = 5, def = 99 }",
            &[],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 99);
}

#[test]
fn otherwise_fallback_expression() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("(10 / 0) otherwise (2 + 3)", &[], &[])
        .unwrap();
    // Primary fails, evaluate fallback expression
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn otherwise_string_type() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"["a", "b"][10] otherwise "default""#, &[], &[])
        .unwrap();
    // Index out of bounds, return fallback
    assert_eq!(result.as_str().unwrap(), "default");
}

#[test]
fn otherwise_float_type() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("(1.0 / 0.0) otherwise 3.14", &[], &[])
        .unwrap();
    // Float division by zero produces inf, not an error
    // So this should return the primary result (inf)
    assert!(result.as_float().unwrap().is_infinite());
}

#[test]
fn otherwise_does_not_catch_stack_overflow() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    // Create a deeply nested expression that will exceed stack depth
    // Use a very small depth limit to trigger overflow quickly
    let mut expr = "1".to_string();
    for _ in 0..50 {
        expr = format!("({expr}) + 1");
    }

    // Add otherwise clause - this should NOT catch the StackOverflow error
    let source = format!("({expr}) otherwise 999");

    let parsed = parser::parse(&arena, &source).unwrap();
    let typed = analyzer::analyze(type_manager, &arena, parsed, &[], &[]).unwrap();

    // Use a very small depth limit to trigger stack overflow
    let result = Evaluator::new(
        EvaluatorOptions { max_depth: 10 },
        &arena,
        type_manager,
        typed,
        &[],
        &[],
    )
    .eval();

    // Should get StackOverflow error, NOT the fallback value
    match result {
        Err(ExecutionError {
            kind: ExecutionErrorKind::ResourceExceeded(ResourceExceededError::StackOverflow { .. }),
            ..
        }) => {
            // Got the expected error - otherwise did not catch it
        }
        Ok(_) => panic!("Expected StackOverflow error, but evaluation succeeded"),
        Err(e) => panic!("Expected StackOverflow error, got: {e:?}"),
    }
}

// ============================================================================
// Cast Tests
// ============================================================================

#[test]
fn cast_int_to_float() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("42 as Float", &[], &[]).unwrap();
    assert_eq!(result.as_float().unwrap(), 42.0);
}

#[test]
fn cast_float_to_int_truncates() {
    let arena = Bump::new();
    // Positive truncation
    let result = Runner::new(&arena).run("3.7 as Int", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 3);
    // Negative truncation
    let result = Runner::new(&arena).run("(-3.7) as Int", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), -3);
}

#[test]
fn cast_str_to_bytes() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#""hello" as Bytes"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_bytes().unwrap(), b"hello");
}

#[test]
fn cast_bytes_to_str_valid_utf8() {
    let arena = Bump::new();
    // First create bytes, then cast back to string
    let result = Runner::new(&arena)
        .run(r#"("hello" as Bytes) as String"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "hello");
}

#[test]
fn cast_bytes_to_str_invalid_utf8() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    // Create invalid UTF-8 bytes via variable
    let invalid_bytes = &[0xFF, 0xFE, 0xFD];
    let bytes_value = Value::bytes(&arena, runner.type_mgr.bytes(), invalid_bytes);

    let var_values = &[("invalid", bytes_value)];
    let result = runner.run("invalid as String", &[], var_values);

    // Should fail with CastError
    assert!(result.is_err());
    match result {
        Err(ExecutionError {
            kind: ExecutionErrorKind::Runtime(RuntimeError::CastError { .. }),
            ..
        }) => {
            // Expected
        }
        _ => panic!("Expected CastError"),
    }
}

#[test]
fn cast_with_otherwise() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    // Create invalid UTF-8 bytes via variable
    let invalid_bytes = &[0xFF, 0xFE, 0xFD];
    let bytes_value = Value::bytes(&arena, runner.type_mgr.bytes(), invalid_bytes);

    let var_values = &[("data", bytes_value)];

    // Use otherwise to handle invalid UTF-8
    let result = runner
        .run(r#"(data as String) otherwise "fallback""#, &[], var_values)
        .unwrap();

    // Should get the fallback value
    assert_eq!(result.as_str().unwrap(), "fallback");
}

#[test]
fn cast_in_expression() {
    let arena = Bump::new();
    // Cast within arithmetic expression
    let result = Runner::new(&arena)
        .run("(42 as Float) + 0.5", &[], &[])
        .unwrap();
    assert_eq!(result.as_float().unwrap(), 42.5);
}

#[test]
fn cast_with_where() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("(x as Float) * 2.0 where { x = 21 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_float().unwrap(), 42.0);
}

#[test]
fn cast_utf8_roundtrip() {
    let arena = Bump::new();
    // String → Bytes → String should preserve unicode
    let result = Runner::new(&arena)
        .run(r#"(("Hello, 世界! 🦀" as Bytes) as String)"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "Hello, 世界! 🦀");
}

// ============================================================================
// FFI Function Calls
// ============================================================================

// Test FFI functions

fn ffi_add<'types, 'arena>(
    ctx: &FfiContext<'types, 'arena>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    assert_eq!(args.len(), 2);
    let a = args[0].as_int().unwrap();
    let b = args[1].as_int().unwrap();
    Ok(Value::int(ctx.type_mgr(), a + b))
}

fn ffi_concat<'types, 'arena>(
    ctx: &FfiContext<'types, 'arena>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    assert_eq!(args.len(), 2);
    let a = args[0].as_str().unwrap();
    let b = args[1].as_str().unwrap();
    let result = ctx.arena().alloc_str(&format!("{a}{b}"));
    Ok(Value::str(ctx.arena(), ctx.type_mgr().str(), result))
}

fn ffi_array_len<'types, 'arena>(
    ctx: &FfiContext<'types, 'arena>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    assert_eq!(args.len(), 1);
    let array = args[0].as_array().unwrap();
    Ok(Value::int(ctx.type_mgr(), array.len() as i64))
}

fn ffi_divide<'types, 'arena>(
    ctx: &FfiContext<'types, 'arena>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    assert_eq!(args.len(), 2);
    let a = args[0].as_int().unwrap();
    let b = args[1].as_int().unwrap();
    if b == 0 {
        // TODO: FFI should return a different error, maybe ExecutionErrorKind?
        return Err(ExecutionError {
            kind: RuntimeError::DivisionByZero {}.into(),
            source: String::new(),
            span: Span(0..0),
        });
    }
    Ok(Value::int(ctx.type_mgr(), a / b))
}

#[test]
fn ffi_simple_call() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let add_ty = runner.type_mgr.function(
        &[runner.type_mgr.int(), runner.type_mgr.int()],
        runner.type_mgr.int(),
    );
    let add_fn = Value::function(&arena, NativeFunction::new(add_ty, ffi_add)).unwrap();

    let result = runner.run("add(10, 32)", &[("add", add_fn)], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn ffi_nested_calls() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let add_ty = runner.type_mgr.function(
        &[runner.type_mgr.int(), runner.type_mgr.int()],
        runner.type_mgr.int(),
    );
    let add_fn = Value::function(&arena, NativeFunction::new(add_ty, ffi_add)).unwrap();

    let result = runner
        .run("add(add(1, 2), 3)", &[("add", add_fn)], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 6);
}

#[test]
fn ffi_string_concat() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let concat_ty = runner.type_mgr.function(
        &[runner.type_mgr.str(), runner.type_mgr.str()],
        runner.type_mgr.str(),
    );
    let concat_fn = Value::function(&arena, NativeFunction::new(concat_ty, ffi_concat)).unwrap();

    let result = runner
        .run(r#"concat("hello", "world")"#, &[("concat", concat_fn)], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "helloworld");
}

#[test]
fn ffi_polymorphic_array_len() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    // len : Array[T] -> Int (polymorphic)
    let len_ty = {
        let t_var = runner.type_mgr.fresh_type_var();
        let array_t = runner.type_mgr.array(t_var);
        runner.type_mgr.function(&[array_t], runner.type_mgr.int())
    };
    let len_fn = Value::function(&arena, NativeFunction::new(len_ty, ffi_array_len)).unwrap();

    let result = runner
        .run("len([1, 2, 3])", &[("len", len_fn)], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 3);
}

#[test]
fn ffi_error_with_otherwise() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let divide_ty = runner.type_mgr.function(
        &[runner.type_mgr.int(), runner.type_mgr.int()],
        runner.type_mgr.int(),
    );
    let divide_fn = Value::function(&arena, NativeFunction::new(divide_ty, ffi_divide)).unwrap();

    let result = runner
        .run("divide(10, 0) otherwise -1", &[("divide", divide_fn)], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), -1);
}

#[test]
fn ffi_call_with_variables() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    let add_ty = runner.type_mgr.function(
        &[runner.type_mgr.int(), runner.type_mgr.int()],
        runner.type_mgr.int(),
    );
    let add_fn = Value::function(&arena, NativeFunction::new(add_ty, ffi_add)).unwrap();

    let result = runner
        .run(
            "add(x, y) where { x = 10, y = 32 }",
            &[("add", add_fn)],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

// ============================================================================
// Lambda Tests (Non-Capturing)
// ============================================================================

#[test]
fn lambda_identity() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("((x) => x)(42)", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn lambda_simple_arithmetic() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("((x) => x + x)(21)", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn lambda_simple_arithmetic_constrained_to_int() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("((x) => x + 1)(21)", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 22);

    let result = Runner::new(&arena)
        .run("((x) => 1 + x)(21)", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 22);
}

#[test]
fn lambda_arithmetic_multiple_params() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("((x, y) => x + y)(10, 20)", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 30);
}

#[test]
fn lambda_two_params_return_array() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("((a, b) => [b, a])(10, 42)", &[], &[])
        .unwrap();
    let array: Vec<_> = result
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e.as_int().unwrap())
        .collect();
    assert_eq!(array, &[42, 10]);
}

#[test]
fn lambda_with_where() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("f(42) where { f = (a) => a }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn lambda_polymorphic() {
    let arena = Bump::new();

    // Test polymorphic identity function with Int
    let result = Runner::new(&arena)
        .run("f(42) where { f = (a) => a }", &[], &[])
        .unwrap();

    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn lambda_nested_call() {
    let arena = Bump::new();
    // Test nested lambdas - inner returns its parameter, outer returns result of calling inner
    let result = Runner::new(&arena)
        .run("((x) => ((y) => y)(42))(100)", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn lambda_as_argument() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    // Create an "apply" function that takes a function and a value
    let apply_ty = {
        let t = runner.type_mgr.fresh_type_var();
        let u = runner.type_mgr.fresh_type_var();
        let func_ty = runner.type_mgr.function(&[t], u);
        runner.type_mgr.function(&[func_ty, t], u)
    };

    fn apply<'types, 'arena>(
        ctx: &FfiContext<'types, 'arena>,
        args: &[Value<'types, 'arena>],
    ) -> Result<Value<'types, 'arena>, ExecutionError> {
        assert_eq!(args.len(), 2);
        let func = args[0].as_function().unwrap();
        let arg = args[1];

        // SAFETY: Type checker guarantees the function accepts the argument type.
        unsafe { func.call_unchecked(ctx, &[arg]) }
    }

    let apply_fn = Value::function(&arena, NativeFunction::new(apply_ty, apply)).unwrap();

    let result = runner
        .run("apply((x) => x, 42)", &[("apply", apply_fn)], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn lambda_with_ffi_abs() {
    let arena = Bump::new();
    let runner = Runner::new(&arena);

    // Create an "abs" function (absolute value)
    let abs_ty = runner
        .type_mgr
        .function(&[runner.type_mgr.int()], runner.type_mgr.int());

    fn ffi_abs<'types, 'arena>(
        ctx: &FfiContext<'types, 'arena>,
        args: &[Value<'types, 'arena>],
    ) -> Result<Value<'types, 'arena>, ExecutionError> {
        assert_eq!(args.len(), 1);
        let val = args[0].as_int().unwrap();
        Ok(Value::int(ctx.type_mgr(), val.abs()))
    }

    let abs_fn = Value::function(&arena, NativeFunction::new(abs_ty, ffi_abs)).unwrap();

    // Create an "apply" function
    let apply_ty = {
        let t = runner.type_mgr.fresh_type_var();
        let u = runner.type_mgr.fresh_type_var();
        let func_ty = runner.type_mgr.function(&[t], u);
        runner.type_mgr.function(&[func_ty, t], u)
    };

    fn apply<'types, 'arena>(
        ctx: &FfiContext<'types, 'arena>,
        args: &[Value<'types, 'arena>],
    ) -> Result<Value<'types, 'arena>, ExecutionError> {
        assert_eq!(args.len(), 2);
        let func = args[0].as_function().unwrap();
        let arg = args[1];
        unsafe { func.call_unchecked(ctx, &[arg]) }
    }

    let apply_fn = Value::function(&arena, NativeFunction::new(apply_ty, apply)).unwrap();

    // Test: apply(abs, -5) where { apply = (f, x) => f(x) }
    let result = runner
        .run(
            "apply(abs, -5)",
            &[("abs", abs_fn), ("apply", apply_fn)],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 5);
}

// ============================================================================
// Closure Tests
// ============================================================================

#[test]
fn closure_simple_capture() {
    let arena = Bump::new();

    // Capture a single variable - lambda just returns the captured value
    let result = Runner::new(&arena)
        .run("f(5) where { x = 10, f = (z) => x }", &[], &[])
        .unwrap();

    assert_eq!(result.as_int().unwrap(), 10);
}

#[test]
fn closure_multiple_captures() {
    let arena = Bump::new();

    // Capture multiple variables - return first capture to verify it was captured
    let result = Runner::new(&arena)
        .run("f(99) where { a = 10, b = 20, f = (x) => a }", &[], &[])
        .unwrap();

    assert_eq!(result.as_int().unwrap(), 10);
}

#[test]
fn closure_nested() {
    let arena = Bump::new();

    // Nested closures - outer captures x, inner also captures x
    let result = Runner::new(&arena)
        .run("f(20)(5) where { x = 42, f = (y) => (z) => x }", &[], &[])
        .unwrap();

    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn closure_returned_from_function() {
    let arena = Bump::new();

    // Function that returns a closure - the closure captures x
    let result = Runner::new(&arena)
        .run(
            "makeAdder(10)(5) where { makeAdder = (x) => (y) => x + y }",
            &[],
            &[],
        )
        .unwrap();

    assert_eq!(result.as_int().unwrap(), 15);
}

#[test]
fn closure_with_where_binding() {
    let arena = Bump::new();

    // Closure captures variable from where binding
    let result = Runner::new(&arena)
        .run(
            "(200 + y) where { x = 10, y = ((z) => 2 * x + z)(2) }",
            &[],
            &[],
        )
        .unwrap();

    assert_eq!(result.as_int().unwrap(), 222);
}

// Additional test cases from lambda-closure-implementation-plan.md

// Milestone 2.2: Simple Function Call Tests
#[test]
fn lambda_zero_params() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("(() => 42)()", &[], &[]).unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

// Milestone 2.3: Closure Call Tests (from original plan)
#[test]
fn closure_capturing_one_variable_inline() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("(((y) => x + y)(5)) where { x = 10 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 15);
}

#[test]
fn closure_capturing_multiple_variables_inline() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            "(((z) => x + y + z)(100)) where { x = 10, y = 20 }",
            &[],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 130);
}

#[test]
fn closure_in_where_binding_multiply() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("f(5) where { f = (x) => x * 2 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 10);
}

// Milestone 2.4: Currying Tests
#[test]
fn simple_currying_inline() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("((x) => (y) => x + y)(10)(20)", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 30);
}

#[test]
fn curried_function_in_where_add() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("add(10)(20) where { add = (x) => (y) => x + y }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 30);
}

#[test]
fn lambda_partial_application() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            "add10(5) where { add = (x) => (y) => x + y, add10 = add(10) }",
            &[],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 15);
}

// Milestone 2.5: Polymorphic Function Tests
#[test]
fn lambda_polymorphic_float() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("id(3.14) where { id = (x) => x }", &[], &[])
        .unwrap();
    assert_eq!(result.as_float().unwrap(), 3.14);
}

#[test]
fn lambda_polymorphic_string() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"id("hello") where { id = (x) => x }"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "hello");
}

#[test]
fn lambda_polymorphic_bool() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("id(true) where { id = (x) => x }", &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn multiple_polymorphic_calls() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            r"{ a = id(42), b = id(3.14) } where { id = (x) => x }",
            &[],
            &[],
        )
        .unwrap();
    let record = result
        .as_record()
        .expect("expression should have returned a record");
    let fields: Vec<_> = record.iter().collect();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].0, "a");
    assert_eq!(fields[0].1.as_int().unwrap(), 42);
    assert_eq!(fields[1].0, "b");
    assert_eq!(fields[1].1.as_float().unwrap(), 3.14);
}

#[test]
fn lambda_array_constructor() {
    // Array construction wrapping a generic parameter with monomorphization
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("wrap(42) where { wrap = (x) => [x] }", &[], &[])
        .unwrap();
    let array: Vec<_> = result
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e.as_int().unwrap())
        .collect();
    assert_eq!(array, &[42]);
}

// Milestone 2.6: Complex Expression Tests
#[test]
fn lambda_with_if_expression() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("((x) => if x > 0 then x else -x)(5)", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn lambda_with_where_in_body() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            "((x) => result where { y = x * 2, result = y + 1 })(5)",
            &[],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 11);
}

#[test]
fn lambda_with_array_in_body() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("((x) => [x, x * 2, x * 3])(10)", &[], &[])
        .unwrap();
    let array: Vec<_> = result
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e.as_int().unwrap())
        .collect();
    assert_eq!(array, &[10, 20, 30]);
}

#[test]
fn lambda_with_format_string() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"((name) => f"Hello, {name}!")("World")"#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "Hello, World!");
}

// Milestone 4.1: Recursive Closure Detection
#[test]
#[ignore = "needs recursive closure detection in analyzer"]
fn recursive_closure_direct_self_reference() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("f(5) where { f = (n) => f(n - 1) }", &[], &[]);
    // Should fail with RecursiveClosure error
    assert!(result.is_err());
}

#[test]
#[ignore = "needs recursive closure detection in analyzer"]
fn recursive_closure_factorial() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run(
        "factorial(5) where { factorial = (n) => if n <= 1 then 1 else n * factorial(n - 1) }",
        &[],
        &[],
    );
    // Should fail with RecursiveClosure error
    assert!(result.is_err());
}

// Milestone 4.3: Edge Cases
#[test]
fn lambda_unused_argument_evaluated() {
    let arena = Bump::new();
    // The argument should be evaluated even if not used in the body
    // This test just verifies the lambda works when argument is unused
    let result = Runner::new(&arena)
        .run("((x) => 42)(100 + 200)", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn lambda_nested_where_shadowing() {
    let arena = Bump::new();
    // Inner where shadows outer x, but lambda should capture outer x
    let result = Runner::new(&arena)
        .run(
            "f(1) where { x = 10, f = (y) => (x + y) where { x = 20 } }",
            &[],
            &[],
        )
        .unwrap();
    // The lambda captures x = 10, then the where binding shadows it with x = 20
    // So it should use the shadowed x = 20
    assert_eq!(result.as_int().unwrap(), 21);
}

#[test]
fn lambda_capturing_lambda() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            "outer(5) where { inner = (x) => x * 2, outer = (y) => inner(y + 1) }",
            &[],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 12); // (5 + 1) * 2 = 12
}

#[test]
fn ord_constraint_on_bool_fails() {
    // This should fail because Bool doesn't implement Ord
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);
    let input = arena.alloc_str("lt(false, true) where { lt = (a, b) => a < b }");

    let parsed = parser::parse(&arena, input).expect("parsing should succeed");
    let result = analyzer::analyze(type_mgr, &arena, parsed, &[], &[]);

    // Should fail during type checking, not during evaluation
    assert!(
        result.is_err(),
        "Expected type checking error for ordering comparison on Bool"
    );

    if let Err(e) = result {
        let error_msg = format!("{e:?}");
        assert!(
            error_msg.contains("Ord") || error_msg.contains("Bool"),
            "Error should mention Ord constraint: {error_msg}"
        );
    }
}

#[test]
fn ord_constraint_on_int_succeeds() {
    // This should succeed because Int implements Ord
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("lt(1, 2) where { lt = (a, b) => a < b }", &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn ord_constraint_on_string_succeeds() {
    // This should succeed because Str implements Ord
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            r#"lt("apple", "banana") where { lt = (a, b) => a < b }"#,
            &[],
            &[],
        )
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn ord_constraint_direct_bool_fails() {
    // Simpler test: directly use < on bools (no lambda abstraction)
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);
    let input = arena.alloc_str("false < true");

    let parsed = parser::parse(&arena, input).expect("parsing should succeed");
    let result = analyzer::analyze(type_mgr, &arena, parsed, &[], &[]);

    // Should fail during type checking
    assert!(
        result.is_err(),
        "Expected type checking error for ordering comparison on Bool (direct)"
    );

    if let Err(e) = result {
        let error_msg = format!("{e:?}");
        assert!(
            error_msg.contains("Ord")
                || error_msg.contains("Bool")
                || error_msg.contains("Ordering"),
            "Error should mention Ord constraint or Bool type: {error_msg}"
        );
    }
}

#[test]
fn numeric_constraint_on_bool_fails() {
    // This should fail because Bool doesn't implement Numeric
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);
    let input = arena.alloc_str("f(false, true) where { f = (a, b) => a + b }");

    let parsed = parser::parse(&arena, input).expect("parsing should succeed");
    let result = analyzer::analyze(type_mgr, &arena, parsed, &[], &[]);

    // Should fail during type checking, not during evaluation
    assert!(
        result.is_err(),
        "Expected type checking error for numeric operation on Bool"
    );

    if let Err(e) = result {
        let error_msg = format!("{e:?}");
        assert!(
            error_msg.contains("Numeric") || error_msg.contains("Bool"),
            "Error should mention Numeric constraint: {error_msg}"
        );
    }
}

#[test]
fn numeric_constraint_direct_bool_fails() {
    // Simpler test: directly use + on bools (no lambda abstraction)
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);
    let input = arena.alloc_str("false + true");

    let parsed = parser::parse(&arena, input).expect("parsing should succeed");
    let result = analyzer::analyze(type_mgr, &arena, parsed, &[], &[]);

    // Should fail during type checking
    assert!(
        result.is_err(),
        "Expected type checking error for numeric operation on Bool (direct)"
    );

    if let Err(e) = result {
        let error_msg = format!("{e:?}");
        assert!(
            error_msg.contains("Numeric")
                || error_msg.contains("Bool")
                || error_msg.contains("Int")
                || error_msg.contains("Float"),
            "Error should mention Numeric constraint or type mismatch: {error_msg}"
        );
    }
}

#[test]
fn numeric_constraint_on_int_succeeds() {
    // This should succeed because Int implements Numeric
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("f(5, 10) where { f = (a, b) => a + b }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 15);
}

// ===== Containment Operator Tests =====

#[test]
fn string_in_string_found() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#""lo" in "hello""#, &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn string_in_string_not_found() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#""x" in "hello""#, &[], &[])
        .unwrap();
    assert!(!result.as_bool().unwrap());
}

#[test]
fn string_not_in_string() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#""x" not in "hello""#, &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn string_in_string_empty_needle() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#""" in "hello""#, &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn bytes_in_bytes_found() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"b"oob" in b"foobar""#, &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn bytes_in_bytes_not_found() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"b"xyz" in b"foobar""#, &[], &[])
        .unwrap();
    assert!(!result.as_bool().unwrap());
}

#[test]
fn bytes_not_in_bytes() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"b"xyz" not in b"foobar""#, &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn bytes_in_bytes_empty_needle() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"b"" in b"foobar""#, &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn int_in_array_found() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("5 in [1, 2, 3, 4, 5]", &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn int_in_array_not_found() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("6 in [1, 2, 3, 4, 5]", &[], &[])
        .unwrap();
    assert!(!result.as_bool().unwrap());
}

#[test]
fn int_not_in_array() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("6 not in [1, 2, 3, 4, 5]", &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn string_in_array_found() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#""foo" in ["foo", "bar", "baz"]"#, &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn element_in_empty_array() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run("1 in []", &[], &[]).unwrap();
    assert!(!result.as_bool().unwrap());
}

#[test]
fn key_in_map_found() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#""key" in {"key": 1, "other": 2}"#, &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn key_in_map_not_found() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#""missing" in {"key": 1, "other": 2}"#, &[], &[])
        .unwrap();
    assert!(!result.as_bool().unwrap());
}

#[test]
fn key_not_in_map() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#""missing" not in {"key": 1, "other": 2}"#, &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn int_key_in_map() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("42 in {42: true, 99: false}", &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn key_in_empty_map() {
    let arena = Bump::new();
    let result = Runner::new(&arena).run(r#""key" in {}"#, &[], &[]).unwrap();
    assert!(!result.as_bool().unwrap());
}

#[test]
fn containment_in_where_binding() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"found where { found = "lo" in "hello" }"#, &[], &[])
        .unwrap();
    assert!(result.as_bool().unwrap());
}

#[test]
fn containment_in_if_condition() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(r#"if 5 in [1, 2, 3, 4, 5] then "yes" else "no""#, &[], &[])
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "yes");
}

// ============================================================================
// Pattern Matching
// ============================================================================

#[test]
fn match_variable_pattern() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("42 match { x -> x }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn match_wildcard_pattern() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("42 match { _ -> 99 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 99);
}

#[test]
fn match_literal_int() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            "5 match { 1 -> \"one\", 2 -> \"two\", 5 -> \"five\", _ -> \"other\" }",
            &[],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_str().unwrap(), "five");
}

#[test]
fn match_literal_bool_true() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("true match { true -> 1, false -> 0 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 1);
}

#[test]
fn match_literal_bool_false() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("false match { true -> 1, false -> 0 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 0);
}

#[test]
fn match_literal_string() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            r#""hello" match { "hi" -> 1, "hello" -> 2, _ -> 3 }"#,
            &[],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 2);
}

#[test]
fn match_option_some() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("some 42 match { some x -> x, none -> 0 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn match_option_none() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("none match { some x -> x, none -> 99 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 99);
}

#[test]
fn match_option_nested_some() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            "some some 5 match { some some x -> x, some none -> -1, none -> 0 }",
            &[],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 5);
}

#[test]
fn match_option_nested_some_none() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            "some none match { some some x -> x, some none -> -1, none -> 0 }",
            &[],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_int().unwrap(), -1);
}

#[test]
fn match_in_where_binding() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            "result where { result = some 10 match { some x -> x * 2, none -> 0 } }",
            &[],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 20);
}

#[test]
fn match_pattern_order() {
    // First matching pattern wins
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("42 match { _ -> 1, 42 -> 2 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 1);
}

#[test]
fn match_with_expression_in_body() {
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run("some 10 match { some x -> x + x, none -> 0 }", &[], &[])
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 20);
}

#[test]
fn match_in_lambda_with_inferable_type() {
    // Type (Option[Int]) => Int is correctly inferred from the body:
    // - Match arms return Int (y * 2 and 0)
    // - Pattern 'some y' with 'y * 2' means y: Int
    // - Patterns 'some y' and 'none' unify x with Option[Int]
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            "f(some 5) where { f = (x) => x match { some y -> y * 2, none -> 0 } }",
            &[],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 10);
}

#[test]
fn match_in_where_with_known_type() {
    // Pattern matching works when types are known from context
    let arena = Bump::new();
    let result = Runner::new(&arena)
        .run(
            "result where { opt = some 5, result = opt match { some y -> y * 2, none -> 0 } }",
            &[],
            &[],
        )
        .unwrap();
    assert_eq!(result.as_int().unwrap(), 10);
}
