use std::sync::Once;

use bumpalo::Bump;

use super::*;
use crate::analyzer::error::{TypeError, TypeErrorKind};
use crate::types::TypeClassId;
use crate::types::manager::TypeManager;
use crate::{format, parser};

// Global tracing initialization for tests
static INIT_TRACING: Once = Once::new();

/// Initialize tracing for tests. Safe to call multiple times.
fn init_tracing() {
    INIT_TRACING.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .try_init();
    });
}

// Helper to parse and analyze a source string
fn analyze_source<'types, 'arena>(
    source: &'arena str,
    type_manager: &'types TypeManager<'types>,
    arena: &'arena Bump,
) -> Result<&'arena typed_expr::TypedExpr<'types, 'arena>, TypeError>
where
    'types: 'arena,
{
    let parsed = parser::parse(arena, source).map_err(|e| {
        TypeError::new(
            TypeErrorKind::Other {
                message: format!("Failed to parse source: {e}"),
            },
            source.to_string(),
            parser::Span::new(0, 0),
        )
    })?;
    analyze(type_manager, arena, parsed, &[], &[])
}

/// Recursively collect all lambda expression pointers from an expression tree.
/// Useful for verifying lambda tracking and pointer remapping.
fn collect_lambda_pointers<'types, 'arena>(
    expr: &typed_expr::Expr<'types, 'arena>,
    lambdas: &mut hashbrown::HashSet<*const typed_expr::Expr<'types, 'arena>>,
) {
    match &expr.1 {
        typed_expr::ExprInner::Lambda { body, .. } => {
            lambdas.insert(std::ptr::from_ref(expr));
            collect_lambda_pointers(body, lambdas);
        }
        typed_expr::ExprInner::Binary { left, right, .. }
        | typed_expr::ExprInner::Boolean { left, right, .. }
        | typed_expr::ExprInner::Comparison { left, right, .. }
        | typed_expr::ExprInner::Index {
            value: left,
            index: right,
        }
        | typed_expr::ExprInner::Otherwise {
            primary: left,
            fallback: right,
        } => {
            collect_lambda_pointers(left, lambdas);
            collect_lambda_pointers(right, lambdas);
        }
        typed_expr::ExprInner::Unary { expr: inner, .. }
        | typed_expr::ExprInner::Field { value: inner, .. }
        | typed_expr::ExprInner::Cast { expr: inner } => {
            collect_lambda_pointers(inner, lambdas);
        }
        typed_expr::ExprInner::Call { callable, args } => {
            collect_lambda_pointers(callable, lambdas);
            for arg in *args {
                collect_lambda_pointers(arg, lambdas);
            }
        }
        typed_expr::ExprInner::If {
            cond,
            then_branch,
            else_branch,
        } => {
            collect_lambda_pointers(cond, lambdas);
            collect_lambda_pointers(then_branch, lambdas);
            collect_lambda_pointers(else_branch, lambdas);
        }
        typed_expr::ExprInner::Where {
            expr: inner,
            bindings,
        } => {
            collect_lambda_pointers(inner, lambdas);
            for (_, value) in *bindings {
                collect_lambda_pointers(value, lambdas);
            }
        }
        typed_expr::ExprInner::Record { fields } => {
            for (_, value) in *fields {
                collect_lambda_pointers(value, lambdas);
            }
        }
        typed_expr::ExprInner::Map { elements } => {
            for (key, value) in *elements {
                collect_lambda_pointers(key, lambdas);
                collect_lambda_pointers(value, lambdas);
            }
        }
        typed_expr::ExprInner::Array { elements } => {
            for elem in *elements {
                collect_lambda_pointers(elem, lambdas);
            }
        }
        typed_expr::ExprInner::FormatStr { exprs, .. } => {
            for expr in *exprs {
                collect_lambda_pointers(expr, lambdas);
            }
        }
        typed_expr::ExprInner::Option { inner } => {
            if let Some(inner_expr) = inner {
                collect_lambda_pointers(inner_expr, lambdas);
            }
        }
        typed_expr::ExprInner::Match { expr, arms } => {
            collect_lambda_pointers(expr, lambdas);
            for arm in *arms {
                collect_lambda_pointers(arm.body, lambdas);
            }
        }
        typed_expr::ExprInner::Constant(_) | typed_expr::ExprInner::Ident(_) => {}
    }
}

// ============================================================================
// Binary Operations
// ============================================================================

#[test]
fn arithmetic_operators_integers() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    for op in ["+", "-", "*", "/", "^"] {
        let source = format!("1 {op} 2");
        let result = analyze_source(&source, type_manager, &bump);
        assert!(result.is_ok(), "Failed for operator {op}");
        assert_eq!(result.unwrap().expr.0, type_manager.int());
    }
}

// ============================================================================
// Type Resolution Tests (Type variables in generic contexts)
// ============================================================================

#[test]
#[ignore = "Type system limitation: generic indexing now defaults to Map for better map support; arrays with Int indexes conflict with Map[Int, V]"]
fn index_in_generic_lambda() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Lambda parameter 'arr' should be constrained to Indexable type class
    // With type classes: ((arr) => arr[0]) :: Indexable a => a -> element_type
    // When called with [1,2,3], unified to Array<Int> -> Int
    let source = "((arr) => arr[0])([1, 2, 3])";
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Index on generic lambda parameter should work with type classes: {result:?}"
    );
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn numeric_constraint_violation_with_source() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // This should fail: trying to add a boolean in a generic lambda
    // The lambda parameter gets unified with Bool, then Numeric constraint fails
    let source = "((x, y) => x + y)(true, false)";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_err(), "Should fail numeric constraint");

    let err = result.unwrap_err();
    // Verify error includes proper error kind
    match &err.kind {
        TypeErrorKind::ConstraintViolation { type_class, .. } => {
            assert_eq!(
                *type_class,
                TypeClassId::Numeric,
                "Error should mention Numeric constraint"
            );
        }
        _ => panic!("Expected ConstraintViolation error, got: {:?}", err.kind),
    }
}

#[test]
fn numeric_int_and_float() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "{ a = f(3, 4), b = f(1.1, 2.2) } where { f = (x, y) => x + y }";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn indexable_lambda_instantiations() {
    use crate::types::type_class::TypeClassId;

    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Lambda with Indexable constraint used with different container types
    let source = r#"{ a = f({1:"one"}, 1), b = f([2, 2], 0) } where { f = (m, k) => m[k] }"#;
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "{result:?}");
    let typed_expr = result.unwrap();

    // Check that lambda instantiations are fully resolved
    assert_eq!(
        typed_expr.lambda_instantiations.len(),
        1,
        "Should have one polymorphic lambda"
    );

    for (_ptr, insts) in &typed_expr.lambda_instantiations {
        assert_eq!(
            insts.substitutions.len(),
            2,
            "Should have two instantiations"
        );

        // Check that the Indexable type class is recorded
        assert!(
            insts.type_classes.contains(&TypeClassId::Indexable),
            "Lambda should have Indexable constraint, got: {:?}",
            insts.type_classes
        );

        // Print the instantiations for debugging
        for (i, subst) in insts.substitutions.iter().enumerate() {
            tracing::debug!("Instantiation {}:", i);
            for (gen_id, ty) in subst {
                tracing::debug!("  {}: {}", gen_id, ty);
                // All types should be fully resolved (no type variables)
                use crate::types::traits::{TypeKind, TypeView};
                assert!(
                    !matches!(ty.view(), TypeKind::TypeVar(_)),
                    "Type variable {gen_id} should be resolved, got: {ty}"
                );
            }
        }
    }
}

#[test]
fn numeric_lambda_type_classes() {
    use crate::types::type_class::TypeClassId;

    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Lambda with Numeric constraint
    let source = "{ a = f(3, 4), b = f(1.1, 2.2) } where { f = (x, y) => x + y }";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "{result:?}");
    let typed_expr = result.unwrap();

    assert_eq!(
        typed_expr.lambda_instantiations.len(),
        1,
        "Should have one polymorphic lambda"
    );

    for (_ptr, insts) in &typed_expr.lambda_instantiations {
        // Check that the Numeric type class is recorded
        assert!(
            insts.type_classes.contains(&TypeClassId::Numeric),
            "Lambda should have Numeric constraint, got: {:?}",
            insts.type_classes
        );
    }
}

#[test]
fn unconstrained_lambda_no_type_classes() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Lambda with no type class constraints (identity function)
    let source = "{ a = f(3), b = f(\"hello\") } where { f = (x) => x }";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "{result:?}");
    let typed_expr = result.unwrap();

    assert_eq!(
        typed_expr.lambda_instantiations.len(),
        1,
        "Should have one polymorphic lambda"
    );

    for (_ptr, insts) in &typed_expr.lambda_instantiations {
        // Unconstrained lambda should have no type classes
        assert!(
            insts.type_classes.is_empty(),
            "Unconstrained lambda should have no type classes, got: {:?}",
            insts.type_classes
        );
    }
}

#[test]
fn nested_indexing_polymorphic_lambda() {
    // crate::test_utils::init_test_logging();
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Lambda with nested indexing - both levels should work with different container types
    let source = r#"{ a = f({true: {"abc": 0.5}}, true, "abc"), b = f({"": [false, true]}, "", 0) } where { f = (m, k1, k2) => m[k1][k2] }"#;
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "{result:?}");
}

#[test]
#[ignore = "Type system limitation: generic indexing now defaults to Map for better map support; arrays with Int indexes conflict with Map[Int, V]"]
fn nested_array_indexing_with_generic() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // This tests that constraint checking handles partially-resolved types correctly
    // The inner array's element type might still be a type variable during constraint check
    // Array[Array[_t]] where _t is later resolved to Int
    // Both Array levels are Indexable regardless of what _t resolves to
    let source = "((arr) => arr[0][0])([[1, 2], [3, 4]])";
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Nested array indexing should work even with partially resolved generic types: {result:?}"
    );
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
#[ignore = "Requires row polymorphism - cannot infer 'any record with field x'"]
fn field_access_in_generic_lambda() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Lambda parameter 'r' should be constrained to "any record with field x"
    // With row polymorphism: ((r) => r.x) :: {x :: Int | r} -> Int
    // When called with {x: 42}, unified to Record{x: Int} -> Int
    let source = "((r) => r.x)({x: 42})";
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Field access on generic lambda parameter should work with row polymorphism: {result:?}"
    );
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
#[ignore = "Requires row polymorphism - nested case"]
fn nested_generic_lambda_field_access() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Higher-order function: pass a record processor function
    // With row polymorphism: f :: {x :: Int | r} -> Int, result :: Int
    // Nested generic lambda composition
    let source = "((f) => f({x = 1}))((r) => r.x)";
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Nested generic lambda with field access should work with row polymorphism: {result:?}"
    );
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
#[ignore = "Cast validation happens during lambda body analysis, before unification"]
fn cast_on_lambda_parameter() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Lambda parameter 'x' should allow cast with delayed validation
    // With constraint system: generate cast constraint, validate after unification
    // When called with 42, x is unified to Int, then Int->Float cast validated
    let source = "((x) => x as Float)(42)";
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Cast on generic lambda parameter should work with delayed validation: {result:?}"
    );
    assert_eq!(result.unwrap().expr.0, type_manager.float());
}

#[test]
fn index_in_where_bound_variable() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // This should work - 'arr' in where clause gets proper type
    // But good regression test in case where-bound variables have similar issues
    let source = "arr[0] where { arr = if true then [1, 2, 3] else [4, 5, 6] }";
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Index on where-bound variable should work: {result:?}"
    );
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn field_access_in_where_bound_variable() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Similar to above - where-bound variable field access
    // Note: Records use '=' not ':' for field assignment
    let source = "r.x where { r = if true then {x = 1} else {x = 2} }";
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Field access on where-bound variable should work: {result:?}"
    );
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn arithmetic_operators_floats() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    for op in ["+", "-", "*", "/", "^"] {
        let source = format!("1.0 {op} 2.0");
        let result = analyze_source(&source, type_manager, &bump);
        assert!(result.is_ok(), "Failed for operator {op}");
        assert_eq!(result.unwrap().expr.0, type_manager.float());
    }
}

#[test]
fn arithmetic_mixed_types_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("1 + 2.0", type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn logical_operators() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    for op in ["and", "or"] {
        let source = format!("true {op} false");
        let result = analyze_source(&source, type_manager, &bump);
        assert!(result.is_ok(), "Failed for operator {op}");
        assert_eq!(result.unwrap().expr.0, type_manager.bool());
    }
}

#[test]
fn logical_operators_non_boolean_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("1 and 2", type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn plus_one_lambda() {
    let arena = Bump::new();
    let type_manager = TypeManager::new(&arena);

    let result = analyze_source(
        "plus_one(9) where { plus_one = (a) => a + 1 }",
        type_manager,
        &arena,
    );
    assert!(result.unwrap().expr.0 == type_manager.int());
}

// ============================================================================
// Type Resolution Pass Tests
// ============================================================================
// These tests verify that the resolve_expr_types pass correctly resolves
// all type variables after constraint finalization.

#[test]
fn type_resolution_simple_call() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Simple polymorphic function call
    // Result should be resolved to concrete type
    let source = r"double(5) where { double = (x) => x * 2 }";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "Analysis should succeed: {result:?}");

    let typed_expr = result.unwrap();

    // Check that the call result has resolved Int type
    if let typed_expr::ExprInner::Where { expr, .. } = &typed_expr.expr.1 {
        assert_eq!(
            expr.0,
            type_manager.int(),
            "Call result should have resolved Int type, not type variable. Got: {:?}",
            expr.0
        );
    } else {
        panic!("Expected Where expression");
    }
}

#[test]
fn type_resolution_array_simple() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Array with multiple polymorphic calls
    // All should resolve to same concrete type
    let source = r"[double(1), double(2)] where { double = (x) => x * 2 }";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "Analysis should succeed: {result:?}");

    let typed_expr = result.unwrap();

    // Check that array elements have resolved Int types
    if let typed_expr::ExprInner::Where { expr, .. } = &typed_expr.expr.1 {
        if let typed_expr::ExprInner::Array { elements } = &expr.1 {
            assert_eq!(elements.len(), 2);

            for (i, elem) in elements.iter().enumerate() {
                assert_eq!(
                    elem.0,
                    type_manager.int(),
                    "Array element {} should have resolved Int type, not type variable. Got: {:?}",
                    i,
                    elem.0
                );
            }
        } else {
            panic!("Expected Array expression");
        }
    } else {
        panic!("Expected Where expression");
    }
}

#[test]
fn type_resolution_map_indexing() {
    // Initialize tracing for this test
    init_tracing();

    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Regression test: Previously, type class constraint result types (like Indexable)
    // were not fully resolved after type checking. This was fixed by the type resolution
    // pass in resolve_expr_types, which replaces all type variables with their concrete
    // types after constraint finalization.
    let source = r#"f({1: "hello"}, 1) where { f = (m, k) => m[k] }"#;
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "Analysis should succeed: {result:?}");

    let typed_expr = result.unwrap();

    // Debug output (enable with RUST_LOG=debug)
    tracing::debug!("=== Expression Tree ===");
    tracing::debug!("{:#?}", typed_expr.expr);

    // Check what the Call expression's type is
    if let typed_expr::ExprInner::Where { expr, .. } = &typed_expr.expr.1 {
        tracing::debug!("=== Call Result Type ===");
        tracing::debug!("Type: {:?}", expr.0);
        tracing::debug!("Expected: {:?}", type_manager.str());

        assert_eq!(
            expr.0,
            type_manager.str(),
            "Map index result should have resolved Str type, not type variable. Got: {:?}",
            expr.0
        );
    } else {
        panic!("Expected Where expression");
    }
}

#[test]
fn type_resolution_nested_structures() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Nested structure with multiple call sites creating type variables
    // All should be resolved after analysis
    let source = r"{a = g(1), b = g(2)} where { g = (n) => n + 10 }";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "Analysis should succeed: {result:?}");

    let typed_expr = result.unwrap();

    // Check that record field expressions have resolved Int types
    if let typed_expr::ExprInner::Where { expr, .. } = &typed_expr.expr.1 {
        if let typed_expr::ExprInner::Record { fields } = &expr.1 {
            assert_eq!(fields.len(), 2);

            for (i, (_, field_expr)) in fields.iter().enumerate() {
                assert_eq!(
                    field_expr.0,
                    type_manager.int(),
                    "Record field {i} should have resolved Int type, not type variable"
                );
            }
        } else {
            panic!("Expected Record expression");
        }
    } else {
        panic!("Expected Where expression");
    }
}

#[test]
fn type_resolution_if_branches() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // If expression with polymorphic calls in both branches
    // Both branches should resolve to same concrete type
    let source = r"
        if true then f(1) else f(2)
        where { f = (x) => x + 10 }
    ";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "Analysis should succeed: {result:?}");

    let typed_expr = result.unwrap();

    // Check that result is Int (not a type variable)
    assert_eq!(
        typed_expr.expr.0,
        type_manager.int(),
        "If expression should have resolved Int type, not type variable"
    );

    // Check that both branches have resolved types
    if let typed_expr::ExprInner::Where { expr, .. } = &typed_expr.expr.1 {
        if let typed_expr::ExprInner::If {
            then_branch,
            else_branch,
            ..
        } = &expr.1
        {
            assert_eq!(
                then_branch.0,
                type_manager.int(),
                "Then branch should have resolved Int type"
            );
            assert_eq!(
                else_branch.0,
                type_manager.int(),
                "Else branch should have resolved Int type"
            );
        } else {
            panic!("Expected If expression");
        }
    } else {
        panic!("Expected Where expression");
    }
}

#[test]
fn type_resolution_empty_array_unification() {
    use crate::types::traits::{TypeKind, TypeView};

    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Simple case: empty array unified with concrete type
    let source = r"if false then [1] else []";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "Analysis should succeed: {result:?}");

    let typed_expr = result.unwrap();

    // The result should be Array[Int]
    match typed_expr.expr.0.view() {
        TypeKind::Array(elem_ty) => {
            assert_eq!(
                elem_ty,
                type_manager.int(),
                "Array element type should be resolved to Int, not type variable. Got: {elem_ty:?}"
            );
        }
        _ => panic!("Expected Array type, got: {:?}", typed_expr.expr.0),
    }
}

#[test]
#[ignore = "Type resolution doesn't fully resolve deeply nested type variables from empty arrays"]
fn type_resolution_deeply_nested_unification() {
    use crate::types::traits::{TypeKind, TypeView};

    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // The else branch creates Array[Array[Array[_0]]] where _0 is a fresh type variable
    // This gets unified with the then branch's Map[Int, Str]
    // Without type resolution, _0 would remain unresolved in the expression tree
    let source = r#"if false then [[[{1:"one"}]]] else [[[]]]"#;
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "Analysis should succeed: {result:?}");

    let typed_expr = result.unwrap();

    // Verify the result type is fully resolved: Array[Array[Array[Map[Int, Str]]]]
    let mut current_ty = typed_expr.expr.0;

    // First level: Array
    match current_ty.view() {
        TypeKind::Array(elem_ty) => current_ty = elem_ty,
        _ => panic!("Expected Array at first level, got: {current_ty:?}"),
    }

    // Second level: Array
    match current_ty.view() {
        TypeKind::Array(elem_ty) => current_ty = elem_ty,
        _ => panic!("Expected Array at second level, got: {current_ty:?}"),
    }

    // Third level: Array
    match current_ty.view() {
        TypeKind::Array(elem_ty) => current_ty = elem_ty,
        _ => panic!("Expected Array at third level, got: {current_ty:?}"),
    }

    // Fourth level: Map[Int, Str] (not a type variable!)
    match current_ty.view() {
        TypeKind::Map(key_ty, val_ty) => {
            assert_eq!(
                key_ty,
                type_manager.int(),
                "Map key should be resolved to Int, not type variable"
            );
            assert_eq!(
                val_ty,
                type_manager.str(),
                "Map value should be resolved to Str, not type variable"
            );
        }
        TypeKind::TypeVar(var_id) => {
            panic!(
                "Innermost type should be Map[Int, Str], not type variable _{var_id}"
            );
        }
        _ => panic!("Expected Map at innermost level, got: {current_ty:?}"),
    }

    // Also verify the else branch expression itself has resolved types
    if let typed_expr::ExprInner::If { else_branch, .. } = &typed_expr.expr.1 {
        // The else branch is [[[]]]
        if let typed_expr::ExprInner::Array { elements: outer } = &else_branch.1 {
            assert_eq!(outer.len(), 1);

            // Second level [[]]
            if let typed_expr::ExprInner::Array { elements: middle } = &outer[0].1 {
                assert_eq!(middle.len(), 1);

                // Third level []
                if let typed_expr::ExprInner::Array { elements: inner } = &middle[0].1 {
                    assert_eq!(inner.len(), 0);

                    // Check the empty array has resolved element type Map[Int, Str]
                    match middle[0].0.view() {
                        TypeKind::Array(elem_ty) => match elem_ty.view() {
                            TypeKind::Map(key_ty, val_ty) => {
                                assert_eq!(
                                    key_ty,
                                    type_manager.int(),
                                    "Empty array element type (key) should be resolved"
                                );
                                assert_eq!(
                                    val_ty,
                                    type_manager.str(),
                                    "Empty array element type (value) should be resolved"
                                );
                            }
                            TypeKind::TypeVar(var_id) => {
                                panic!(
                                    "Empty array element type should be Map[Int, Str], not _{var_id}"
                                );
                            }
                            _ => panic!("Expected Map element type, got: {elem_ty:?}"),
                        },
                        _ => panic!("Expected Array type"),
                    }
                } else {
                    panic!("Expected innermost Array expression");
                }
            } else {
                panic!("Expected middle Array expression");
            }
        } else {
            panic!("Expected outer Array expression");
        }
    } else {
        panic!("Expected If expression");
    }
}

#[test]
fn type_resolution_polymorphic_calls() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Polymorphic lambda called multiple times with same type
    // All call sites should have resolved types (not type variables)
    let source = r"[id(1), id(2), id(3)] where { id = (x) => x }";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "Analysis should succeed: {result:?}");

    let typed_expr = result.unwrap();

    // The array elements should have resolved Int types
    if let typed_expr::ExprInner::Where { expr, .. } = &typed_expr.expr.1 {
        if let typed_expr::ExprInner::Array { elements } = &expr.1 {
            assert_eq!(elements.len(), 3);

            for (i, elem) in elements.iter().enumerate() {
                assert_eq!(
                    elem.0,
                    type_manager.int(),
                    "Array element {i} should have resolved Int type, not type variable"
                );
            }
        } else {
            panic!("Expected Array expression");
        }
    } else {
        panic!("Expected Where expression");
    }
}

#[test]
fn type_resolution_map_construction() {
    use crate::types::traits::{TypeKind, TypeView};

    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Map with polymorphic function calls as values
    let source = r"
        {1: double(5), 2: double(10)}
        where { double = (x) => x * 2 }
    ";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "Analysis should succeed: {result:?}");

    let typed_expr = result.unwrap();

    // Check that result is Map[Int, Int] with resolved types
    match typed_expr.expr.0.view() {
        TypeKind::Map(key_ty, val_ty) => {
            assert_eq!(
                key_ty,
                type_manager.int(),
                "Map key type should be resolved to Int"
            );
            assert_eq!(
                val_ty,
                type_manager.int(),
                "Map value type should be resolved to Int"
            );
        }
        _ => panic!("Expected Map type, got: {:?}", typed_expr.expr.0),
    }
}

// ============================================================================
// Unary Operations
// ============================================================================

#[test]
fn unary_negation() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Note: -42 is a negative literal, not negation. Need -(42) to test the operator
    let result = analyze_source("-(42)", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());

    let result = analyze_source("-(3.14)", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.float());

    // Also test with a variable to ensure it works on non-literals
    let result = analyze_source("-x where { x = 5 }", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn unary_negation_non_numeric_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("-(true)", type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn unary_not() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("not true", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.bool());
}

#[test]
fn unary_not_non_boolean_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("not 42", type_manager, &bump);
    assert!(result.is_err());
}

// ============================================================================
// Literals
// ============================================================================

#[test]
fn literals() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let int_result = analyze_source("42", type_manager, &bump);
    assert!(int_result.is_ok());
    assert_eq!(int_result.unwrap().expr.0, type_manager.int());

    let float_result = analyze_source("3.14", type_manager, &bump);
    assert!(float_result.is_ok());
    assert_eq!(float_result.unwrap().expr.0, type_manager.float());

    let bool_result = analyze_source("true", type_manager, &bump);
    assert!(bool_result.is_ok());
    assert_eq!(bool_result.unwrap().expr.0, type_manager.bool());

    let str_result = analyze_source("\"hello\"", type_manager, &bump);
    assert!(str_result.is_ok());
    assert_eq!(str_result.unwrap().expr.0, type_manager.str());
}

#[test]
fn all_literal_types() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source(
        "{ int = 42, float = 3.14, bool = false, str = \"foo\", bytes = b\"bar\" }",
        type_manager,
        &bump,
    );
    assert!(result.is_ok());
    let result = result.unwrap();

    let expected_type = type_manager.record(vec![
        ("int", type_manager.int()),
        ("float", type_manager.float()),
        ("bool", type_manager.bool()),
        ("str", type_manager.str()),
        ("bytes", type_manager.bytes()),
    ]);

    assert_eq!(result.expr.0, expected_type);
}

// ============================================================================
// Cast
// ============================================================================

#[test]
fn cast_identity_allowed() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Identity casts are allowed (they're just no-ops)
    let result = analyze_source("42 as Int", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());

    let result = analyze_source("\"hello\" as String", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.str());

    let result = analyze_source("3.14 as Float", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.float());
}

#[test]
fn cast_unknown_type_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("42 as Foo", type_manager, &bump);
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_string = format!("{err:?}");
    assert!(err_string.contains("Unknown type: Foo"));
}

// ============================================================================
// Variables and Scopes
// ============================================================================

#[test]
fn undefined_variable_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("undefined_var", type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn where_binding() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("x where { x = 42 }", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn where_duplicate_binding_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("x where { x = 1, x = 2 }", type_manager, &bump);
    assert!(result.is_err());
}

// ============================================================================
// Lambdas and Functions
// ============================================================================

#[test]
fn lambda_basic() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("(x) => x", type_manager, &bump);
    assert!(result.is_ok());

    // Check it's a function type
    match result.unwrap().expr.0 {
        crate::types::Type::Function { .. } => {}
        _ => panic!("Expected function type"),
    }
}

#[test]
fn lambda_duplicate_parameter_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("(x, x) => x", type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn function_call() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("((x) => x)(42)", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn call_non_function_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("42()", type_manager, &bump);
    assert!(result.is_err());
}

// ============================================================================
// If Expressions
// ============================================================================

#[test]
fn if_expression() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("if true then 1 else 2", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn if_non_boolean_condition_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("if 1 then 2 else 3", type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn if_mismatched_branches_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("if true then 1 else false", type_manager, &bump);
    assert!(result.is_err());
}

// ============================================================================
// Arrays
// ============================================================================

#[test]
fn array_homogeneous() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("[1, 2, 3]", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().expr.0,
        type_manager.array(type_manager.int())
    );
}

#[test]
fn array_heterogeneous_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("[1, true]", type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn array_empty() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("[]", type_manager, &bump);
    assert!(result.is_ok());

    match result.unwrap().expr.0 {
        crate::types::Type::Array(..) => {}
        _ => panic!("Expected array type"),
    }
}

// ============================================================================
// Incomplete Features (marked with #[ignore])
// ============================================================================

#[test]
fn array_indexing() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("[1, 2, 3][0]", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn array_indexing_with_variable() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source(
        "arr[i] where { arr = [1, 2, 3], i = 0 }",
        type_manager,
        &bump,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn array_indexing_non_integer_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("[1, 2, 3][true]", type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn indexing_non_indexable_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("42[0]", type_manager, &bump);
    assert!(result.is_err());
}

// ============================================================================
// Record Tests
// ============================================================================

#[test]
fn record_empty() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("Record{}", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.record(vec![]));
}

#[test]
fn record_single_field() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ x = 42 }", type_manager, &bump);
    assert!(result.is_ok());
    let result = result.unwrap();
    let expected = type_manager.record(vec![("x", type_manager.int())]);
    assert_eq!(result.expr.0, expected);
}

#[test]
fn record_multiple_fields() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ x = 42, y = true, z = \"hello\" }", type_manager, &bump);
    assert!(result.is_ok());
    let result = result.unwrap();
    let expected = type_manager.record(vec![
        ("x", type_manager.int()),
        ("y", type_manager.bool()),
        ("z", type_manager.str()),
    ]);
    assert_eq!(result.expr.0, expected);
}

// ============================================================================
// Field Access Tests
// ============================================================================

#[test]
fn record_field_access() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ x = 42 }.x", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn record_field_access_multiple_fields() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ x = 42, y = \"hello\" }.y", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.str());
}

#[test]
fn record_field_access_nonexistent_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ x = 42 }.y", type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn field_access_non_record_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("42.x", type_manager, &bump);
    assert!(result.is_err());
}

// ============================================================================
// Map Tests
// ============================================================================

#[test]
fn map_empty() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{}", type_manager, &bump);
    assert!(result.is_ok());
    match result.unwrap().expr.0 {
        crate::types::Type::Map(..) => {}
        _ => panic!("Expected map type"),
    }
}

#[test]
fn map_homogeneous_types() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ \"a\": 1, \"b\": 2 }", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().expr.0,
        type_manager.map(type_manager.str(), type_manager.int())
    );
}

#[test]
fn map_heterogeneous_keys_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ \"a\": 1, 2: 3 }", type_manager, &bump);
    let err = result.unwrap_err();
    // Should fail with type mismatch error (heterogeneous keys)
    assert!(matches!(err.kind, TypeErrorKind::TypeMismatch { .. }));
}

#[test]
fn map_heterogeneous_values_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("{ \"a\": 1, \"b\": true }", type_manager, &bump);
    let err = result.unwrap_err();
    // Should fail with type mismatch error (heterogeneous values)
    assert!(matches!(err.kind, TypeErrorKind::TypeMismatch { .. }));
}

// ============================================================================
// FormatStr Tests
// ============================================================================

#[test]
fn format_str_no_interpolations() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("f\"hello\"", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.str());
}

#[test]
fn format_str_with_interpolations() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("f\"x = {x}\" where { x = 42 }", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.str());
}

#[test]
fn format_str_function_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source(
        "f\"func = {f}\" where { f = (x) => x }",
        type_manager,
        &bump,
    );
    assert!(result.is_err());
}

#[test]
fn otherwise_same_types() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("1 otherwise 2", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.int());
}

#[test]
fn otherwise_type_mismatch_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // This should fail because Int and Str don't match
    let result = analyze_source("1 otherwise \"error\"", type_manager, &bump);
    assert!(result.is_err());
}

#[test]
fn otherwise_with_array_indexing() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // This represents: array[0] otherwise "default" where array = ["foo"]
    // For now just test compatible types work
    let result = analyze_source("\"foo\" otherwise \"bar\"", type_manager, &bump);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().expr.0, type_manager.str());
}

// ============================================================================
// Cast Tests
// ============================================================================

#[test]
fn cast_invalid() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Int → Str is not supported (use format strings instead)
    let result = analyze_source("42 as Str", type_manager, &bump);
    assert!(result.is_err());
    // Should fail because Int → Str is not a valid cast
}

// ============================================================================
// Literal Suffix Tests
// ============================================================================

#[test]
fn integer_suffix_not_supported() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("42`MB`", type_manager, &bump);
    assert_eq!(
        result.unwrap_err().kind,
        TypeErrorKind::UnsupportedFeature {
            feature: "Integer suffixes are not yet supported".to_string(),
            suggestion: "In the future, suffixes will support units of measurement (e.g., 10`MB`, 5`seconds`)".to_string(),
        }
    );
}

// ============================================================================
// Span Tracking
// ============================================================================

#[test]
fn span_tracking_binary_expr() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "1 + 2";
    let result = analyze_source(source, type_manager, &bump).unwrap();

    // Verify we have the annotation
    assert_eq!(result.ann.source, "1 + 2");

    // Check root expression span (the whole "1 + 2")
    let root_span = result.ann.span_of(result.expr);
    assert_eq!(root_span, Some(parser::Span::new(0, 5)));

    // Check sub-expressions have spans
    if let typed_expr::ExprInner::Binary { left, right, .. } = &result.expr.1 {
        // Left operand "1" should be at position 0..1
        let left_span = result.ann.span_of(left);
        assert_eq!(left_span, Some(parser::Span::new(0, 1)));

        // Right operand "2" should be at position 4..5
        let right_span = result.ann.span_of(right);
        assert_eq!(right_span, Some(parser::Span::new(4, 5)));
    } else {
        panic!("Expected Binary expression");
    }
}

#[test]
#[ignore = "Span tracking logic needs to be fixed in parser"]
fn span_tracking_nested_expr() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "(1 + 2) * 3";
    let result = analyze_source(source, type_manager, &bump).unwrap();

    // Root multiplication should span from first operand to last (0..11)
    // Note: there is currently a bug in the span tracking logic.
    assert_eq!(
        result.ann.span_of(result.expr),
        Some(parser::Span::new(0, 11))
    );

    // Verify nested expressions also have spans
    if let typed_expr::ExprInner::Binary { left, right, .. } = &result.expr.1 {
        // Left addition "(1 + 2)" spans the content inside parens: 1..6
        let left_span = result.ann.span_of(left);
        assert_eq!(left_span, Some(parser::Span::new(1, 6)));

        // Right "3" should be 10..11
        let right_span = result.ann.span_of(right);
        assert_eq!(right_span, Some(parser::Span::new(10, 11)));
    } else {
        panic!("Expected Binary expression");
    }
}

#[test]
fn span_tracking_boolean_expr() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "true and false";
    let result = analyze_source(source, type_manager, &bump).unwrap();

    // Root should span the whole expression
    assert_eq!(
        result.ann.span_of(result.expr),
        Some(parser::Span::new(0, 14))
    );

    // Verify operands have spans
    if let typed_expr::ExprInner::Boolean { left, right, .. } = &result.expr.1 {
        // Left "true" should be 0..4
        let left_span = result.ann.span_of(left);
        assert_eq!(left_span, Some(parser::Span::new(0, 4)));

        // Right "false" should be 9..14
        let right_span = result.ann.span_of(right);
        assert_eq!(right_span, Some(parser::Span::new(9, 14)));
    } else {
        panic!("Expected Boolean expression");
    }
}

#[test]
fn float_suffix_not_supported() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let result = analyze_source("3.14`meters`", type_manager, &bump);
    assert_eq!(
        result.unwrap_err().kind,
        TypeErrorKind::UnsupportedFeature {
            feature: "Float suffixes are not yet supported".to_string(),
            suggestion: "In the future, suffixes will support units of measurement (e.g., 3.14`meters`, 2.5`kg`)".to_string(),
        }
    );
}

// ============================================================================
// Polymorphic Let Bindings (Type Schemes)
// ============================================================================

#[test]
fn polymorphic_identity_function() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // The identity function should work with both Int and Str
    let source = "{ a = id(1), b = id(\"foo\") } where { id = (x) => x }";
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Polymorphic identity function should typecheck: {result:?}"
    );

    let typed = result.unwrap();
    assert_eq!(
        typed.expr.0,
        type_manager.record(vec![("a", type_manager.int()), ("b", type_manager.str())])
    );
}

#[test]
fn polymorphic_inline_lambda() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Inline lambda with polymorphic parameters
    let source = "((a, b) => [b, a])(10, 42)";
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Polymorphic inline lambda should typecheck: {result:?}"
    );

    // The result should be an Array[Int]
    let typed = result.unwrap();
    assert_eq!(typed.expr.0, type_manager.array(type_manager.int()));
}

#[test]
fn polymorphic_pair_function() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // pair function should work with different types (but arrays are homogeneous)
    let source = r#"
        {
            int_pair = pair(1, 2),
            str_pair = pair("a", "b"),
            bool_pair = pair(true, false)
        }
        where {
            pair = (x, y) => [x, y]
        }
    "#;
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Polymorphic pair function should typecheck: {result:?}"
    );
}

#[test]
fn polymorphic_const_function() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // const function: (x, y) => x (returns first argument, ignores second)
    let source = r#"
        {
            a = konst(42, "ignored"),
            b = konst("hello", true)
        }
        where {
            konst = (x, y) => x
        }
    "#;
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Polymorphic const function should typecheck: {result:?}"
    );

    let typed = result.unwrap();
    assert_eq!(
        typed.expr.0,
        type_manager.record(vec![("a", type_manager.int()), ("b", type_manager.str())])
    );
}

#[test]
fn sequential_polymorphic_bindings() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Sequential bindings where later bindings can use earlier polymorphic ones
    let source = r#"
        {
            id_result1 = id(42),
            id_result2 = id("hello"),
            wrap_result = wrap(id(99)),
        } where {
            id = (x) => x,
            wrap = (x) => [x],
        }
    "#;
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Sequential polymorphic bindings should typecheck: {result:?}"
    );

    let typed = result.unwrap();
    assert_eq!(
        typed.expr.0,
        type_manager.record(vec![
            ("id_result1", type_manager.int()),
            ("id_result2", type_manager.str()),
            ("wrap_result", type_manager.array(type_manager.int())),
        ])
    );
}

#[test]
fn higher_rank_polymorphism() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // This requires passing a polymorphic function as an argument
    // Currently fails because we can't pass type schemes as values
    let source = r"
        apply(id, 42) where {
            id = (x) => x,
            apply = (f, x) => f(x)
        }
    ";
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Higher-rank polymorphism should typecheck: {result:?}"
    );

    let typed = result.unwrap();
    assert_eq!(typed.expr.0, type_manager.int());
}

#[test]
fn polymorphic_in_array_literal() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Array containing results of polymorphic function calls
    let source = r"
        [id(1), id(2), id(3)] where { id = (x) => x }
    ";
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Polymorphic function in array literal should typecheck: {result:?}"
    );

    let typed = result.unwrap();
    assert_eq!(typed.expr.0, type_manager.array(type_manager.int()));
}

#[test]
fn nested_where_with_polymorphism() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Nested where clauses, inner scope uses outer polymorphic binding
    let source = r#"
        result where {
            id = (x) => x,
            result = inner where {
                inner = { a = id(1), b = id("test") }
            }
        }
    "#;
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Nested where with polymorphism should typecheck: {result:?}"
    );
}

#[test]
fn polymorphic_function_type_error() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // This should fail: trying to use id with inconsistent types in same context
    // where unification is required
    let source = r#"
        [id(1), id("mixed")] where { id = (x) => x }
    "#;
    let result = analyze_source(source, type_manager, &bump);

    // Arrays are homogeneous, so id(1) fixes the array element type to Int,
    // then id("mixed") should fail because Str != Int
    assert!(
        result.is_err(),
        "Mixed types in homogeneous array should fail"
    );
}

#[test]
fn polymorphic_map_function() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Simple map-like function (not actual iteration, just demonstrates polymorphism)
    // Fails because apply_twice needs to accept a polymorphic function parameter
    let source = r#"
        {
            int_result = apply_twice((x) => x + 1, 5),
            str_result = apply_twice((s) => s, "hello")
        }
        where {
            apply_twice = (f, x) => f(f(x))
        }
    "#;
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Polymorphic apply_twice should typecheck: {result:?}"
    );

    let typed = result.unwrap();
    assert_eq!(
        typed.expr.0,
        type_manager.record(vec![
            ("int_result", type_manager.int()),
            ("str_result", type_manager.str()),
        ])
    );
}

#[test]
fn polymorphic_compose() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Function composition with polymorphism
    // Type checks successfully but type variables aren't fully resolved
    let source = r#"
        {
            result1 = wrap(1),
            result2 = wrap("test")
        } where {
            id = (x) => x,
            wrap = (x) => [id(x)]
        }
    "#;
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Polymorphic composition should typecheck: {result:?}"
    );

    let typed = result.unwrap();
    assert_eq!(
        typed.expr.0,
        type_manager.record(vec![
            ("result1", type_manager.array(type_manager.int())),
            ("result2", type_manager.array(type_manager.str())),
        ])
    );
}

#[test]
fn closure_capturing_lambda_param_should_not_be_polymorphic() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // This tests that closures capturing lambda parameters are NOT polymorphic.
    // The parameter p has type Bool (from calling with true).
    // capture = () => p should have type () => Bool, NOT be polymorphic.
    // Therefore capture() + 1 should fail (can't add Bool + Int).
    let source = r"
        ((p) => result where {
          capture = () => p,
          result = capture() + 1
        })(true)
    ";
    let result = analyze_source(source, type_manager, &bump);

    // Should fail with specific type mismatch: Bool vs Int
    match result {
        Err(err) => {
            let err_str = format!("{err:?}");
            assert!(
                err_str.contains("TypeMismatch")
                    && (err_str.contains("Bool") && err_str.contains("Int")),
                "Expected TypeMismatch between Bool and Int, got: {err:?}"
            );
        }
        Ok(_) => panic!(
            "Expected type error: closure capturing lambda parameter should not be polymorphic"
        ),
    }
}

// ============================================================================
// Comprehensive Error Message Tests
// ============================================================================

#[test]
fn error_unbound_variable() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "unknown_var";
    let result = analyze_source(source, type_manager, &bump);

    match result {
        Err(err) => {
            let diagnostic = err.to_diagnostic();
            assert_eq!(diagnostic.code, Some("E002".to_string()));
            assert!(
                diagnostic
                    .message
                    .contains("Undefined variable 'unknown_var'")
            );
            assert!(!diagnostic.help.is_empty());
        }
        Ok(_) => panic!("Expected UnboundVariable error"),
    }
}

#[test]
fn error_not_indexable() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "true[0]";
    let result = analyze_source(source, type_manager, &bump);

    match result {
        Err(err) => {
            let diagnostic = err.to_diagnostic();
            assert_eq!(diagnostic.code, Some("E009".to_string()));
            assert!(
                diagnostic
                    .message
                    .contains("Cannot index into non-indexable type")
            );
            assert!(diagnostic.message.contains("Bool"));
        }
        Ok(_) => panic!("Expected NotIndexable error"),
    }
}

#[test]
fn error_unknown_field() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "{ a = 1, b = 2 }.c";
    let result = analyze_source(source, type_manager, &bump);

    match result {
        Err(err) => {
            let diagnostic = err.to_diagnostic();
            assert_eq!(diagnostic.code, Some("E010".to_string()));
            assert!(
                diagnostic
                    .message
                    .contains("Record does not have field 'c'")
            );
            assert!(diagnostic.message.contains("Available fields"));
            assert!(diagnostic.message.contains('a') || diagnostic.message.contains('b'));
        }
        Ok(_) => panic!("Expected UnknownField error"),
    }
}

#[test]
fn error_not_a_record() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Access field on an array (not a record)
    let source = "[1, 2, 3].field";
    let result = analyze_source(source, type_manager, &bump);

    match result {
        Err(err) => {
            let diagnostic = err.to_diagnostic();
            assert_eq!(diagnostic.code, Some("E012".to_string()));
            assert!(diagnostic.message.contains("Cannot access field"));
            assert!(diagnostic.message.contains("non-record type"));
        }
        Ok(_) => panic!("Expected NotARecord error"),
    }
}

#[test]
fn error_duplicate_parameter() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "(x, x) => x + 1";
    let result = analyze_source(source, type_manager, &bump);

    match result {
        Err(err) => {
            let diagnostic = err.to_diagnostic();
            assert_eq!(diagnostic.code, Some("E015".to_string()));
            assert!(diagnostic.message.contains("Duplicate parameter name 'x'"));
        }
        Ok(_) => panic!("Expected DuplicateParameter error"),
    }
}

#[test]
fn error_duplicate_binding() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "x + y where { x = 1, x = 2 }";
    let result = analyze_source(source, type_manager, &bump);

    match result {
        Err(err) => {
            let diagnostic = err.to_diagnostic();
            assert_eq!(diagnostic.code, Some("E016".to_string()));
            assert!(diagnostic.message.contains("Duplicate binding name 'x'"));
        }
        Ok(_) => panic!("Expected DuplicateBinding error"),
    }
}

#[test]
fn error_not_formattable() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = r#"f"Value: {func}" where { func = (x) => x }"#;
    let result = analyze_source(source, type_manager, &bump);

    match result {
        Err(err) => {
            let diagnostic = err.to_diagnostic();
            assert_eq!(diagnostic.code, Some("E017".to_string()));
            assert!(diagnostic.message.contains("Cannot format"));
            assert!(diagnostic.message.contains("format string"));
        }
        Ok(_) => panic!("Expected NotFormattable error"),
    }
}

#[test]
fn error_constraint_violation_numeric() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // This triggers TypeMismatch (E001) because both are concrete types
    // To get ConstraintViolation, we need a type variable with a Numeric constraint
    // For now, just check that we get a type error
    let source = "\"text\" + 1";
    let result = analyze_source(source, type_manager, &bump);

    match result {
        Err(err) => {
            let diagnostic = err.to_diagnostic();
            tracing::debug!(
                code = ?diagnostic.code,
                message = %diagnostic.message,
                "Got error"
            );
            // This actually gives E001 (TypeMismatch) not E005 (ConstraintViolation)
            // because Str and Int are concrete types that don't match
            assert_eq!(diagnostic.code, Some("E001".to_string()));
            assert!(diagnostic.message.contains("Type mismatch"));
        }
        Ok(_) => panic!("Expected type error"),
    }
}

#[test]
fn error_constraint_violation_ord() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "[1, 2] < [3, 4]";
    let result = analyze_source(source, type_manager, &bump);

    match result {
        Err(err) => {
            let diagnostic = err.to_diagnostic();
            assert_eq!(diagnostic.code, Some("E005".to_string()));
            assert!(diagnostic.message.contains("does not implement Ord"));
        }
        Ok(_) => panic!("Expected ConstraintViolation error"),
    }
}

#[test]
fn error_type_mismatch_binary_op() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "1 + \"text\"";
    let result = analyze_source(source, type_manager, &bump);

    match result {
        Err(err) => {
            let diagnostic = err.to_diagnostic();
            assert_eq!(diagnostic.code, Some("E001".to_string()));
            assert!(diagnostic.message.contains("Type mismatch"));
            assert!(diagnostic.message.contains("Int"));
            assert!(diagnostic.message.contains("Str"));
        }
        Ok(_) => panic!("Expected TypeMismatch error"),
    }
}

#[test]
fn error_function_param_count_mismatch() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "((x, y) => x + y)(1)";
    let result = analyze_source(source, type_manager, &bump);

    match result {
        Err(err) => {
            let diagnostic = err.to_diagnostic();
            assert_eq!(diagnostic.code, Some("E008".to_string()));
            assert!(
                diagnostic
                    .message
                    .contains("Function parameter count mismatch")
            );
            assert!(diagnostic.message.contains("expected 2"));
            assert!(diagnostic.message.contains("found 1"));
        }
        Ok(_) => panic!("Expected FunctionParamCountMismatch error"),
    }
}

#[test]
fn error_invalid_cast() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Try to cast array to int (invalid)
    let source = "[1, 2, 3] as Int";
    let result = analyze_source(source, type_manager, &bump);

    match result {
        Err(err) => {
            let diagnostic = err.to_diagnostic();
            assert_eq!(diagnostic.code, Some("E014".to_string()));
            assert!(diagnostic.message.contains("Cannot cast"));
        }
        Ok(_) => panic!("Expected InvalidCast error"),
    }
}

#[test]
fn error_polymorphic_cast() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Try to cast a polymorphic value (lambda parameter)
    let source = "f(1) where { f = (x) => x as Float }";
    let result = analyze_source(source, type_manager, &bump);

    match result {
        Err(err) => {
            let diagnostic = err.to_diagnostic();
            assert_eq!(diagnostic.code, Some("E019".to_string()));
            assert!(diagnostic.message.contains("Cannot cast polymorphic value"));
            assert!(diagnostic.message.contains("Float"));
            // Should have 2 help messages
            assert_eq!(diagnostic.help.len(), 2);
            assert!(diagnostic.help[0].contains("not yet supported"));
            assert!(diagnostic.help[1].contains("concrete type"));
            // Verify context is present showing where type was inferred
            assert_eq!(diagnostic.related.len(), 1);
            assert!(diagnostic.related[0].message.contains("inferred here"));
        }
        Ok(_) => panic!("Expected PolymorphicCast error"),
    }
}

#[test]
fn error_unsupported_feature_integer_suffix() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "100`MB`";
    let result = analyze_source(source, type_manager, &bump);

    match result {
        Err(err) => {
            let diagnostic = err.to_diagnostic();
            assert_eq!(diagnostic.code, Some("E018".to_string()));
            assert!(diagnostic.message.contains("Integer suffixes"));
            assert!(
                diagnostic
                    .help
                    .iter()
                    .any(|h| h.contains("units of measurement"))
            );
        }
        Ok(_) => panic!("Expected UnsupportedFeature error"),
    }
}

#[test]
fn error_unsupported_feature_float_suffix() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "3.14`meters`";
    let result = analyze_source(source, type_manager, &bump);

    match result {
        Err(err) => {
            let diagnostic = err.to_diagnostic();
            assert_eq!(diagnostic.code, Some("E018".to_string()));
            assert!(diagnostic.message.contains("Float suffixes"));
            assert!(
                diagnostic
                    .help
                    .iter()
                    .any(|h| h.contains("units of measurement"))
            );
        }
        Ok(_) => panic!("Expected UnsupportedFeature error"),
    }
}

#[test]
fn lambda_body_type_variables_after_resolution() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Lambda stored in where binding - body has type variables _0, _1, _2
    let source = r"id where { id = (x) => x }";

    let result = analyze_source(source, type_manager, &bump);
    assert!(result.is_ok());

    let typed_expr = result.unwrap();

    // Print the full tree to see what the lambda body looks like
    tracing::debug!("\n=== Full Expression Tree ===");
    tracing::debug!("{:#?}", typed_expr.expr);

    // Extract the lambda from the where binding
    if let typed_expr::ExprInner::Where { bindings, .. } = &typed_expr.expr.1 {
        let (_name, lambda_expr) = bindings[0];
        if let typed_expr::ExprInner::Lambda { body, .. } = &lambda_expr.1 {
            tracing::debug!("\n=== Lambda Body ===");
            tracing::debug!("Body type: {:?}", body.0);
            tracing::debug!("Body expression: {:#?}", body);

            // The body should still have _0 because it's a generalized type variable
            // It was never unified with anything concrete
            // Use the public TypeView trait to check if it's a TypeVar
            use crate::types::traits::{TypeKind, TypeView};
            assert!(matches!(body.0.view(), TypeKind::TypeVar(_)));
        } else {
            panic!("Expected Lambda expression in binding");
        }
    } else {
        panic!("Expected Where expression");
    }
}

#[test]
fn array_lambda_body_unified_types() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Lambda with array - requires a and b to unify
    let source = r"f(10, 42) where { f = (a, b) => [b, a] }";

    let result = analyze_source(source, type_manager, &bump);
    assert!(result.is_ok());

    let typed_expr = result.unwrap();

    tracing::debug!("\n=== Full Tree ===");
    tracing::debug!("{:#?}", typed_expr.expr);

    // Extract the lambda from where binding
    if let typed_expr::ExprInner::Where { bindings, .. } = &typed_expr.expr.1 {
        let (_name, lambda_expr) = bindings[0];
        if let typed_expr::ExprInner::Lambda { params, body, .. } = &lambda_expr.1 {
            tracing::debug!("\n=== Lambda ===");
            tracing::debug!("Lambda type: {:?}", lambda_expr.0);
            tracing::debug!("Params: {:?}", params);
            tracing::debug!("Body type: {:?}", body.0);

            // Look at the array inside
            if let typed_expr::ExprInner::Array { elements } = &body.1 {
                tracing::debug!("\n=== Array Elements ===");
                tracing::debug!("Element 0 (b) type: {:?}", elements[0].0);
                tracing::debug!("Element 1 (a) type: {:?}", elements[1].0);

                // Check if they're the same pointer
                let same_ptr = core::ptr::eq(elements[0].0, elements[1].0);
                tracing::debug!("Same type pointer: {}", same_ptr);

                // During analysis, a and b should have been unified
                // After type resolution, they should point to the same Type
                assert!(
                    same_ptr,
                    "Array elements must have pointer-equal types after unification and resolution.\n\
                     Element 0 (b) type: {:?}\n\
                     Element 1 (a) type: {:?}\n\
                     This is required for array construction to work in the evaluator.",
                    elements[0].0, elements[1].0
                );
            } else {
                panic!("Expected Array expression in lambda body");
            }
        } else {
            panic!("Expected Lambda expression in binding");
        }
    } else {
        panic!("Expected Where expression");
    }
}

#[test]
fn inline_array_lambda_works() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Inline version - this works
    let source = r"((a, b) => [b, a])(10, 42)";

    let result = analyze_source(source, type_manager, &bump);
    assert!(result.is_ok());

    let typed_expr = result.unwrap();

    tracing::debug!("\n=== Inline Lambda Tree ===");
    tracing::debug!("{:#?}", typed_expr.expr);

    // Check the call expression
    if let typed_expr::ExprInner::Call { callable, .. } = &typed_expr.expr.1 {
        if let typed_expr::ExprInner::Lambda { body, .. } = &callable.1 {
            // Look at the array inside
            if let typed_expr::ExprInner::Array { elements } = &body.1 {
                tracing::debug!("\n=== Inline Array Body ===");
                tracing::debug!("Array type: {:?}", body.0);
                tracing::debug!("Element 0 type: {:?}", elements[0].0);
                tracing::debug!("Element 1 type: {:?}", elements[1].0);

                // Check that elements are resolved to concrete Int types, not type variables
                use crate::types::traits::{TypeKind, TypeView};
                assert!(
                    matches!(elements[0].0.view(), TypeKind::Int),
                    "Element 0 should have resolved Int type, got {:?}",
                    elements[0].0
                );
                assert!(
                    matches!(elements[1].0.view(), TypeKind::Int),
                    "Element 1 should have resolved Int type, got {:?}",
                    elements[1].0
                );
            } else {
                panic!("Expected Array expression in lambda body");
            }
        } else {
            panic!("Expected Lambda expression as callable");
        }
    } else {
        panic!("Expected Call expression");
    }
}

// ============================================================================
// Lambda Instantiation Tracking Tests (Phase 2)
// ============================================================================

#[test]
fn instantiation_tracking_simple() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Simple polymorphic lambda with one call site
    let source = r"f(10) where { f = (x) => x }";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok());
    let typed_expr = result.unwrap();

    // Should have exactly one lambda with instantiations tracked
    assert_eq!(
        typed_expr.lambda_instantiations.len(),
        1,
        "Should track instantiations for 1 polymorphic lambda"
    );

    // Get the lambda instantiation info
    let inst_info = typed_expr.lambda_instantiations.values().next().unwrap();

    // Should have exactly one instantiation (the call to f(10))
    assert_eq!(
        inst_info.substitutions.len(),
        1,
        "Should have 1 instantiation"
    );

    // Check the substitution maps to Int (don't depend on specific var ID)
    let subst = &inst_info.substitutions[0];
    assert_eq!(subst.len(), 1, "Should have 1 mapping");

    // The single mapped value should be Int
    let concrete_ty = subst.values().next().expect("Should have one mapping");
    assert_eq!(
        *concrete_ty,
        type_manager.int(),
        "Type variable should map to Int"
    );
}

#[test]
fn instantiation_tracking_multiple_calls() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Polymorphic lambda called with different types (using record to allow mixed types)
    let source = r#"{a = f(10), b = f("hello")} where { f = (x) => x }"#;
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok());
    let typed_expr = result.unwrap();

    // Should have one lambda tracked
    assert_eq!(typed_expr.lambda_instantiations.len(), 1);

    let inst_info = typed_expr.lambda_instantiations.values().next().unwrap();

    // Should have two instantiations
    assert_eq!(
        inst_info.substitutions.len(),
        2,
        "Should have 2 instantiations (one for Int, one for Str)"
    );

    // Check that we have both Int and Str instantiations (order-independent)
    let mut saw_int = false;
    let mut saw_str = false;
    for subst in &inst_info.substitutions {
        for ty in subst.values() {
            if *ty == type_manager.int() {
                saw_int = true;
            }
            if *ty == type_manager.str() {
                saw_str = true;
            }
        }
    }
    assert!(saw_int, "Should have Int instantiation");
    assert!(saw_str, "Should have Str instantiation");
}

#[test]
fn instantiation_tracking_multi_param() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Lambda with multiple parameters
    let source = r"f(10, 42) where { f = (a, b) => [b, a] }";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok());
    let typed_expr = result.unwrap();

    assert_eq!(typed_expr.lambda_instantiations.len(), 1);

    let inst_info = typed_expr.lambda_instantiations.values().next().unwrap();
    assert_eq!(inst_info.substitutions.len(), 1);

    // Both parameters should map to Int
    // (They get unified because array requires same type)
    let subst = &inst_info.substitutions[0];

    // The lambda has type scheme ∀[0,1]. (_0, _1) -> Array[_1] where _0 = _1
    // So we should see mappings for the quantified variables
    assert!(
        !subst.is_empty(),
        "Should have at least 1 mapping for the unified type variable"
    );

    // Check that the mapped types are Int
    for (var_id, concrete_ty) in subst {
        assert_eq!(
            *concrete_ty,
            type_manager.int(),
            "Var {var_id} should map to Int"
        );
    }
}

#[test]
fn instantiation_tracking_map_indexing() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Polymorphic map indexing with multiple key types
    let source = r#"[f({1: "one"}, 1), f({"two": "dos"}, "two")] where { f = (m, k) => m[k] }"#;
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok());
    let typed_expr = result.unwrap();

    assert_eq!(typed_expr.lambda_instantiations.len(), 1);

    let inst_info = typed_expr.lambda_instantiations.values().next().unwrap();

    // Should have two instantiations
    assert_eq!(
        inst_info.substitutions.len(),
        2,
        "Should have 2 instantiations (Map[Int,Str] and Map[Str,Str])"
    );

    // Both instantiations should map to Str for the result type
    // (because both maps have Str values)
    for subst in &inst_info.substitutions {
        // Find the result type variable in the substitution
        // One of the variables should map to Str (the result type)
        let has_str = subst.values().any(|ty| *ty == type_manager.str());
        assert!(
            has_str,
            "Each instantiation should have Str in the substitution"
        );
    }
}

#[test]
fn no_instantiation_for_monomorphic() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Monomorphic lambda - usage constrains it to a specific type (Int)
    let source = r"f(10) where { f = (x) => x + 1 }";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok());
    let typed_expr = result.unwrap();

    // Should not track instantiations for monomorphic lambdas
    assert_eq!(
        typed_expr.lambda_instantiations.len(),
        0,
        "Should not track monomorphic lambdas"
    );
}

#[test]
fn no_instantiation_for_inline_lambda() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Inline lambda - not bound in a where clause
    let source = r"((x) => x)(10)";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok());
    let typed_expr = result.unwrap();

    // Should not track inline lambdas (only where-bound ones)
    assert_eq!(
        typed_expr.lambda_instantiations.len(),
        0,
        "Should not track inline lambdas"
    );
}

#[test]
fn instantiation_tracking_with_shadowing() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Test case with nested scopes and shadowing
    // Inner f = (x) => x is shadowed by outer f = g
    // g uses the inner f twice with different types
    // The outer f (which is g) is used twice with different types
    let source = r#"
        {first = f(1), second = f("hello")} where {
            f = g where {
                f = (x) => x,
                g = (y) => [f(y), f(y)]
            }
        }
    "#;
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Should analyze successfully: {:?}",
        result.err()
    );
    let typed_expr = result.unwrap();

    // The inner f = (x) => x is used twice in [f(y), f(y)], but both uses must have
    // the same type due to array homogeneity, so they unify to 1 instantiation
    // The outer f = g is instantiated twice: f(1) and f("hello")
    // This test verifies that shadowing works correctly - the inner f doesn't interfere
    // with tracking the outer f

    // Should have 2 lambdas tracked
    assert_eq!(
        typed_expr.lambda_instantiations.len(),
        2,
        "Should track both the inner identity lambda and g"
    );

    // Find which lambda has which number of instantiations
    let mut inst_counts: Vec<usize> = typed_expr
        .lambda_instantiations
        .values()
        .map(|info| info.substitutions.len())
        .collect();
    inst_counts.sort_unstable();

    // Inner f: 1 instantiation (both uses in array unified to same type)
    // Outer f (g): 2 instantiations (used with Int and Str)
    assert_eq!(
        inst_counts,
        vec![1, 2],
        "Should have correct instantiation counts"
    );
}

#[test]
fn lambda_instantiations_pointer_remapping() {
    // Regression test for pointer invalidation bug:
    // resolve_expr_types allocates new Expr nodes, so lambda_instantiations
    // keys must be remapped from old pointers to new pointers.
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Use a map indexing example like the original test
    let source = r#"[f({1: "one"}, 1), f({"two": "2"}, "two")] where { f = (m, k) => m[k] }"#;
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Analysis should succeed: {:?}",
        result.err()
    );
    let typed_expr = result.unwrap();

    // Should have tracked instantiations
    assert_eq!(
        typed_expr.lambda_instantiations.len(),
        1,
        "Should track instantiations for 1 polymorphic lambda"
    );

    // Collect all lambda pointers from the resolved expression tree
    let mut tree_lambdas = hashbrown::HashSet::new();
    collect_lambda_pointers(typed_expr.expr, &mut tree_lambdas);

    // Verify that every key in lambda_instantiations exists in the resolved tree
    for lambda_ptr in typed_expr.lambda_instantiations.keys() {
        assert!(
            tree_lambdas.contains(lambda_ptr),
            "lambda_instantiations key must point to a lambda in the resolved tree.\n\
             This would fail if resolve_expr_types allocated new nodes but didn't remap the keys."
        );
    }

    // Should have found the polymorphic lambda
    assert_eq!(tree_lambdas.len(), 1, "Should have 1 lambda in the tree");
}

// ============================================================================
// Option Type Tests
// ============================================================================

#[test]
fn some_literal_int() {
    use crate::types::traits::{TypeKind, TypeView};

    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "some 1";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "Analysis should succeed: {result:?}");

    let typed_expr = result.unwrap();

    // The result should be Option[Int]
    match typed_expr.expr.0.view() {
        TypeKind::Option(inner_ty) => {
            assert_eq!(
                inner_ty,
                type_manager.int(),
                "Option inner type should be Int. Got: {inner_ty:?}"
            );
        }
        _ => panic!("Expected Option type, got: {:?}", typed_expr.expr.0),
    }
}

#[test]
fn none_polymorphic() {
    use crate::types::traits::{TypeKind, TypeView};

    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "none";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "Analysis should succeed: {result:?}");

    let typed_expr = result.unwrap();

    // The result should be Option[_0] (type variable)
    match typed_expr.expr.0.view() {
        TypeKind::Option(inner_ty) => {
            match inner_ty.view() {
                TypeKind::TypeVar(_) => {
                    // Expected: polymorphic none
                }
                _ => panic!("Option inner should be type variable, got: {inner_ty:?}"),
            }
        }
        _ => panic!("Expected Option type, got: {:?}", typed_expr.expr.0),
    }
}

#[test]
fn if_none_some_string() {
    use crate::types::traits::{TypeKind, TypeView};

    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = r#"if true then none else some "foo""#;
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "Analysis should succeed: {result:?}");

    let typed_expr = result.unwrap();

    // The result should be Option[String]
    // none gets unified with some "foo" to get Option[String]
    match typed_expr.expr.0.view() {
        TypeKind::Option(inner_ty) => {
            assert_eq!(
                inner_ty,
                type_manager.str(),
                "Option inner type should be String after unification. Got: {inner_ty:?}"
            );
        }
        _ => panic!("Expected Option type, got: {:?}", typed_expr.expr.0),
    }
}

#[test]
fn option_in_lambda() {
    use crate::types::traits::{TypeKind, TypeView};

    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "f(true) where { f = (x) => some x }";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "Analysis should succeed: {result:?}");

    let typed_expr = result.unwrap();

    // The result should be Option[Bool]
    // f has type Bool -> Option[Bool], applied to true
    match typed_expr.expr.0.view() {
        TypeKind::Option(inner_ty) => {
            assert_eq!(
                inner_ty,
                type_manager.bool(),
                "Option inner type should be Bool. Got: {inner_ty:?}"
            );
        }
        _ => panic!("Expected Option type, got: {:?}", typed_expr.expr.0),
    }
}

#[test]
fn array_of_options() {
    use crate::types::traits::{TypeKind, TypeView};

    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "[none, none, none, some 3.14]";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "Analysis should succeed: {result:?}");

    let typed_expr = result.unwrap();

    // The result should be Array[Option[Float]]
    match typed_expr.expr.0.view() {
        TypeKind::Array(elem_ty) => match elem_ty.view() {
            TypeKind::Option(inner_ty) => {
                assert_eq!(
                    inner_ty,
                    type_manager.float(),
                    "Option inner type should be Float. Got: {inner_ty:?}"
                );
            }
            _ => panic!("Array element should be Option type, got: {elem_ty:?}"),
        },
        _ => panic!("Expected Array type, got: {:?}", typed_expr.expr.0),
    }
}

#[test]
fn nested_option() {
    use crate::types::traits::{TypeKind, TypeView};

    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = "some some 42";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "Analysis should succeed: {result:?}");

    let typed_expr = result.unwrap();

    // The result should be Option[Option[Int]]
    match typed_expr.expr.0.view() {
        TypeKind::Option(outer_inner) => match outer_inner.view() {
            TypeKind::Option(inner_inner) => {
                assert_eq!(
                    inner_inner,
                    type_manager.int(),
                    "Innermost type should be Int. Got: {inner_inner:?}"
                );
            }
            _ => panic!("Expected nested Option type, got: {outer_inner:?}"),
        },
        _ => panic!("Expected Option type, got: {:?}", typed_expr.expr.0),
    }
}

#[test]
fn option_in_record() {
    use crate::types::traits::{TypeKind, TypeView};

    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    let source = r"{ x = some 42, y = none }";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_ok(), "Analysis should succeed: {result:?}");

    let typed_expr = result.unwrap();

    // The result should be Record[x: Option[Int], y: Option[_0]]
    match typed_expr.expr.0.view() {
        TypeKind::Record(fields) => {
            let fields_vec: Vec<_> = fields.collect();
            assert_eq!(fields_vec.len(), 2, "Record should have 2 fields");

            // Check x field
            let (x_name, x_ty) = fields_vec[0];
            assert_eq!(x_name, "x");
            match x_ty.view() {
                TypeKind::Option(inner) => {
                    assert_eq!(inner, type_manager.int());
                }
                _ => panic!("Field x should be Option[Int], got: {x_ty:?}"),
            }

            // Check y field
            let (y_name, y_ty) = fields_vec[1];
            assert_eq!(y_name, "y");
            match y_ty.view() {
                TypeKind::Option(_) => {
                    // Polymorphic none, type variable is fine
                }
                _ => panic!("Field y should be Option type, got: {y_ty:?}"),
            }
        }
        _ => panic!("Expected Record type, got: {:?}", typed_expr.expr.0),
    }
}

#[test]
fn exhaustiveness_option_with_catch_all() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // This should be exhaustive: some _ and none
    let source = r"some(42) match { some x -> x, none -> 0 }";
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Should be exhaustive with catch-all pattern: {result:?}"
    );
}

#[test]
fn exhaustiveness_option_specific_pattern_fails() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // This should NOT be exhaustive: some 1 only matches Some(1), not all Some values
    let source = r"some(42) match { some 1 -> 1, none -> 0 }";
    let result = analyze_source(source, type_manager, &bump);

    assert!(result.is_err(), "Should fail: some 1 is not exhaustive");

    if let Err(err) = result {
        let diagnostic = err.to_diagnostic();
        assert!(
            diagnostic.message.contains("Non-exhaustive patterns"),
            "Error should be about non-exhaustive patterns"
        );
        assert!(
            diagnostic.message.contains("Option"),
            "Error should mention Option type"
        );
    }
}

#[test]
fn exhaustiveness_nested_option_with_catch_all() {
    let bump = Bump::new();
    let type_manager = TypeManager::new(&bump);

    // Nested pattern with catch-all should be exhaustive
    let source = r"some(some(42)) match { some (some x) -> x, some none -> 0, none -> 0 }";
    let result = analyze_source(source, type_manager, &bump);

    assert!(
        result.is_ok(),
        "Should be exhaustive with nested catch-all: {result:?}"
    );
}
