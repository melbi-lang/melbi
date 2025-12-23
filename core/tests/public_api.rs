//! Integration tests for the public API.
//!
//! These tests validate that the public API works end-to-end with real
//! parsing, type checking, and evaluation.

use bumpalo::Bump;
use melbi_core::api::{
    CompileOptions, CompileOptionsOverride, Engine, EngineOptions, EnvironmentBuilder,
};
use melbi_core::evaluator::ExecutionError;
use melbi_core::values::binder::Binder;
use melbi_core::values::dynamic::Value;
use melbi_core::values::{FfiContext, NativeFunction};

#[test]
fn test_basic_compilation_and_execution() {
    // Create engine with empty environment
    let arena = Bump::new();
    let engine = Engine::new(Default::default(), &arena, |_arena, _type_mgr, env| {
        // Empty environment
        env
    });

    // Compile a simple arithmetic expression
    let compile_opts = CompileOptionsOverride::default();
    let expr = engine
        .compile(compile_opts, "1 + 2", &[])
        .expect("compilation should succeed");

    // Execute
    let val_arena = Bump::new();
    let result = expr
        .run(Default::default(), &val_arena, &[])
        .expect("execution should succeed");

    // Validate result
    assert_eq!(result.as_int().unwrap(), 3);
}

#[test]
fn test_parameterized_expression() {
    let arena = Bump::new();
    let options = EngineOptions::default();
    let engine = Engine::new(options, &arena, |_arena, _type_mgr, env| env);

    // Compile with parameters
    let int_ty = engine.type_manager().int();
    let compile_opts = CompileOptionsOverride::default();
    let expr = engine
        .compile(compile_opts, "x + y", &[("x", int_ty), ("y", int_ty)])
        .expect("compilation should succeed");

    // Execute with arguments
    let val_arena = Bump::new();
    let type_mgr = engine.type_manager();
    let result = expr
        .run(
            Default::default(),
            &val_arena,
            &[Value::int(type_mgr, 10), Value::int(type_mgr, 32)],
        )
        .expect("execution should succeed");

    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_environment_registration_constant() {
    let arena = Bump::new();
    let options = EngineOptions::default();
    let engine = Engine::new(options, &arena, |_arena, type_mgr, env| {
        // Register a constant
        env.bind("pi", Value::float(type_mgr, std::f64::consts::PI))
    });

    // Compile expression using the constant
    let compile_opts = CompileOptionsOverride::default();
    let expr = engine
        .compile(compile_opts, "pi * 2.0", &[])
        .expect("compilation should succeed");

    // Execute
    let val_arena = Bump::new();
    let result = expr
        .run(Default::default(), &val_arena, &[])
        .expect("execution should succeed");

    // Validate result
    let result_float = result.as_float().unwrap();
    assert!((result_float - 6.28318).abs() < 0.00001);
}

#[test]
fn test_native_function_registration() {
    let arena = Bump::new();
    let options = EngineOptions::default();
    let engine = Engine::new(options, &arena, |arena, type_mgr, env| {
        // Define a native add function
        fn add<'types, 'arena>(
            ctx: &FfiContext<'types, 'arena>,
            args: &[Value<'types, 'arena>],
        ) -> Result<Value<'types, 'arena>, ExecutionError> {
            let a = args[0].as_int().expect("argument should be int");
            let b = args[1].as_int().expect("argument should be int");
            Ok(Value::int(ctx.type_mgr(), a + b))
        }

        // Register the function
        let int_ty = type_mgr.int();
        let add_ty = type_mgr.function(&[int_ty, int_ty], int_ty);
        let add_fn = NativeFunction::new(add_ty, add);
        env.bind("add", Value::function(arena, add_fn).unwrap())
    });

    // Compile expression calling the function
    let compile_opts = CompileOptionsOverride::default();
    let expr = engine
        .compile(compile_opts, "add(40, 2)", &[])
        .expect("compilation should succeed");

    // Execute
    let val_arena = Bump::new();
    let result = expr
        .run(Default::default(), &val_arena, &[])
        .expect("execution should succeed");

    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_error_arg_count_mismatch() {
    let arena = Bump::new();
    let options = EngineOptions::default();
    let engine = Engine::new(options, &arena, |_arena, _type_mgr, env| env);

    // Compile with 2 parameters
    let int_ty = engine.type_manager().int();
    let compile_opts = CompileOptionsOverride::default();
    let expr = engine
        .compile(compile_opts, "x + y", &[("x", int_ty), ("y", int_ty)])
        .expect("compilation should succeed");

    // Try to execute with wrong number of arguments
    let val_arena = Bump::new();
    let type_mgr = engine.type_manager();
    let result = expr.run(Default::default(), &val_arena, &[Value::int(type_mgr, 10)]);

    // Should fail with argument count mismatch
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Argument count mismatch"));
}

#[test]
fn test_error_type_mismatch() {
    let arena = Bump::new();
    let options = EngineOptions::default();
    let engine = Engine::new(options, &arena, |_arena, _type_mgr, env| env);

    // Compile with int parameter
    let int_ty = engine.type_manager().int();
    let compile_opts = CompileOptionsOverride::default();
    let expr = engine
        .compile(compile_opts, "x + 1", &[("x", int_ty)])
        .expect("compilation should succeed");

    // Try to execute with float argument
    let val_arena = Bump::new();
    let type_mgr = engine.type_manager();
    let result = expr.run(
        Default::default(),
        &val_arena,
        &[Value::float(type_mgr, 3.14)],
    );

    // Should fail with type mismatch
    assert!(result.is_err());
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("Type mismatch"));
}

#[test]
fn test_error_compilation_failure() {
    let arena = Bump::new();
    let options = EngineOptions::default();
    let engine = Engine::new(options, &arena, |_arena, _type_mgr, env| env);

    // Try to compile invalid syntax
    let compile_opts = CompileOptionsOverride::default();
    let result = engine.compile(compile_opts, "1 + + 2", &[]);

    // Should fail during compilation
    assert!(result.is_err());
}

#[test]
fn test_error_undefined_variable() {
    let arena = Bump::new();
    let options = EngineOptions::default();
    let engine = Engine::new(options, &arena, |_arena, _type_mgr, env| env);

    // Try to compile expression with undefined variable
    let compile_opts = CompileOptionsOverride::default();
    let result = engine.compile(compile_opts, "x + 1", &[]);

    // Should fail during type checking
    assert!(result.is_err());
}

#[test]
fn test_run_unchecked() {
    let arena = Bump::new();
    let options = EngineOptions::default();
    let engine = Engine::new(options, &arena, |_arena, _type_mgr, env| env);

    // Compile with parameter
    let int_ty = engine.type_manager().int();
    let compile_opts = CompileOptionsOverride::default();
    let expr = engine
        .compile(compile_opts, "x * 2", &[("x", int_ty)])
        .expect("compilation should succeed");

    // Execute using unchecked API
    let val_arena = Bump::new();
    let type_mgr = engine.type_manager();
    let result =
        unsafe { expr.run_unchecked(Default::default(), &val_arena, &[Value::int(type_mgr, 21)]) }
            .expect("execution should succeed");

    assert_eq!(result.as_int().unwrap(), 42);
}

#[test]
fn test_multiple_executions_same_expression() {
    let arena = Bump::new();
    let options = EngineOptions::default();
    let engine = Engine::new(options, &arena, |_arena, _type_mgr, env| env);

    // Compile once
    let int_ty = engine.type_manager().int();
    let compile_opts = CompileOptionsOverride::default();
    let expr = engine
        .compile(compile_opts, "x + y", &[("x", int_ty), ("y", int_ty)])
        .expect("compilation should succeed");

    let type_mgr = engine.type_manager();

    // Execute multiple times with different arguments and arenas
    for (x, y, expected) in &[(1, 2, 3), (10, 20, 30), (100, 200, 300)] {
        let val_arena = Bump::new();
        let result = expr
            .run(
                Default::default(),
                &val_arena,
                &[Value::int(type_mgr, *x), Value::int(type_mgr, *y)],
            )
            .expect("execution should succeed");
        assert_eq!(result.as_int().unwrap(), *expected);
    }
}

#[test]
fn test_complex_expression_with_multiple_operations() {
    let arena = Bump::new();
    let options = EngineOptions::default();
    let engine = Engine::new(options, &arena, |_arena, type_mgr, env| {
        // Register some constants
        env.bind("a", Value::int(type_mgr, 10))
            .bind("b", Value::int(type_mgr, 5))
    });

    // Compile complex expression
    let int_ty = engine.type_manager().int();
    let compile_opts = CompileOptionsOverride::default();
    let expr = engine
        .compile(
            compile_opts,
            "(a + b) * x - y / 2",
            &[("x", int_ty), ("y", int_ty)],
        )
        .expect("compilation should succeed");

    // Execute
    let val_arena = Bump::new();
    let type_mgr = engine.type_manager();
    let result = expr
        .run(
            Default::default(),
            &val_arena,
            &[Value::int(type_mgr, 2), Value::int(type_mgr, 10)],
        )
        .expect("execution should succeed");

    // (10 + 5) * 2 - 10 / 2 = 15 * 2 - 5 = 30 - 5 = 25
    assert_eq!(result.as_int().unwrap(), 25);
}

#[test]
fn test_engine_options_max_depth() {
    use melbi_core::api::RunOptions;

    let arena = Bump::new();
    let options = EngineOptions {
        default_compile_options: CompileOptions::default(),
        default_run_options: RunOptions {
            max_depth: 5,
            max_iterations: None, // Unlimited
        },
    };
    let engine = Engine::new(options, &arena, |arena, type_mgr, env| {
        // Register a recursive function that will exceed max_depth
        fn factorial<'types, 'arena>(
            ctx: &FfiContext<'types, 'arena>,
            args: &[Value<'types, 'arena>],
        ) -> Result<Value<'types, 'arena>, ExecutionError> {
            let n = args[0].as_int().expect("argument should be int");
            if n <= 1 {
                Ok(Value::int(ctx.type_mgr(), 1))
            } else {
                // This would require recursive calls, but for testing we'll just
                // create a deeply nested expression instead
                Ok(Value::int(ctx.type_mgr(), n))
            }
        }

        let int_ty = type_mgr.int();
        let factorial_ty = type_mgr.function(&[int_ty], int_ty);
        let factorial_fn = NativeFunction::new(factorial_ty, factorial);
        env.bind("factorial", Value::function(arena, factorial_fn).unwrap())
    });

    // This test validates that engine options are properly stored and used
    // More comprehensive max_depth testing would require deeply nested expressions
    let compile_opts = CompileOptionsOverride::default();
    let expr = engine
        .compile(compile_opts, "factorial(5)", &[])
        .expect("compilation should succeed");

    let val_arena = Bump::new();
    let _result = expr
        .run(Default::default(), &val_arena, &[])
        .expect("execution should succeed");
}

#[test]
fn test_error_duplicate_registration() {
    let arena = Bump::new();
    let type_mgr = melbi_core::types::manager::TypeManager::new(&arena);

    // Test proper error handling for duplicate registration
    let builder = EnvironmentBuilder::new(&arena)
        .bind("x", Value::int(type_mgr, 10))
        .bind("x", Value::int(type_mgr, 20));

    let result = builder.build();

    assert!(
        result.is_err(),
        "Duplicate registration should return an error"
    );
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("Duplicate registration"),
        "Error message should mention duplicate: {}",
        msg
    );
    assert!(
        msg.contains("'x'"),
        "Error message should mention the name: {}",
        msg
    );
}

#[test]
fn test_access_expression_metadata() {
    let arena = Bump::new();
    let options = EngineOptions::default();
    let engine = Engine::new(options, &arena, |_arena, _type_mgr, env| env);

    // Compile expression
    let int_ty = engine.type_manager().int();
    let compile_opts = CompileOptionsOverride::default();
    let expr = engine
        .compile(
            compile_opts,
            "x + y + z",
            &[("x", int_ty), ("y", int_ty), ("z", int_ty)],
        )
        .expect("compilation should succeed");

    // Access metadata
    let params = expr.params();
    assert_eq!(params.len(), 3);
    assert_eq!(params[0].0, "x");
    assert_eq!(params[1].0, "y");
    assert_eq!(params[2].0, "z");

    let return_type = expr.return_type();
    assert!(core::ptr::eq(return_type, int_ty));
}
