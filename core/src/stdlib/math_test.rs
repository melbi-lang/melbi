//! Tests for the Math package

use super::{register_math_functions, register_math_package};
use crate::{
    api::{CompileOptionsOverride, Engine, EngineOptions},
    types::manager::TypeManager, // This import is necessary for test helpers
    values::{
        binder::Binder,
        dynamic::{RecordBuilder, Value},
    },
};
use bumpalo::Bump;

#[test]
fn test_math_package_builds() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let math = register_math_functions(&arena, type_mgr, RecordBuilder::new(&arena, type_mgr))
        .build()
        .unwrap();
    let record = math.as_record().unwrap();

    // Should have all constants and functions
    assert!(!record.is_empty());
}

#[test]
fn test_math_constants() {
    let arena = Bump::new();
    let type_mgr = TypeManager::new(&arena);

    let math = register_math_functions(&arena, type_mgr, RecordBuilder::new(&arena, type_mgr))
        .build()
        .unwrap();
    let record = math.as_record().unwrap();

    // Test PI
    let pi = record.get("PI").unwrap();
    assert_eq!(pi.as_float().unwrap(), core::f64::consts::PI);

    // Test E
    let e = record.get("E").unwrap();
    assert_eq!(e.as_float().unwrap(), core::f64::consts::E);

    // Test TAU
    let tau = record.get("TAU").unwrap();
    assert_eq!(tau.as_float().unwrap(), core::f64::consts::TAU);

    // Test INFINITY
    let infinity = record.get("INFINITY").unwrap();
    assert!(infinity.as_float().unwrap().is_infinite());
    assert!(infinity.as_float().unwrap().is_sign_positive());

    // Test NAN
    let nan = record.get("NAN").unwrap();
    assert!(nan.as_float().unwrap().is_nan());
}

// Helper function for integration tests using the Engine to evaluate Melbi code
fn test_math_expr<F>(source: &str, check: F)
where
    F: FnOnce(Value),
{
    let options = EngineOptions::default();
    let arena = Bump::new();

    let engine = Engine::new(options, &arena, register_math_package);

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
fn test_math_abs() {
    test_math_expr("Math.Abs(3.14)", |r: Value| {
        assert_eq!(r.as_float().unwrap(), 3.14);
    });

    test_math_expr("Math.Abs(-3.14)", |r: Value| {
        assert_eq!(r.as_float().unwrap(), 3.14);
    });

    test_math_expr("Math.Abs(0.0)", |r: Value| {
        assert_eq!(r.as_float().unwrap(), 0.0);
    });
}

#[test]
fn test_math_min_max() {
    test_math_expr("Math.Min(3.0, 5.0)", |r: Value| {
        assert_eq!(r.as_float().unwrap(), 3.0);
    });

    test_math_expr("Math.Max(3.0, 5.0)", |r: Value| {
        assert_eq!(r.as_float().unwrap(), 5.0);
    });
}

#[test]
fn test_math_clamp() {
    test_math_expr("Math.Clamp(5.0, 0.0, 10.0)", |r: Value| {
        assert_eq!(r.as_float().unwrap(), 5.0);
    });

    test_math_expr("Math.Clamp(-5.0, 0.0, 10.0)", |r: Value| {
        assert_eq!(r.as_float().unwrap(), 0.0);
    });

    test_math_expr("Math.Clamp(15.0, 0.0, 10.0)", |r: Value| {
        assert_eq!(r.as_float().unwrap(), 10.0);
    });
}

#[test]
fn test_math_rounding() {
    test_math_expr("Math.Floor(3.7)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 3);
    });

    test_math_expr("Math.Floor(-3.7)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), -4);
    });

    test_math_expr("Math.Ceil(3.2)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 4);
    });

    test_math_expr("Math.Round(3.6)", |r: Value| {
        assert_eq!(r.as_int().unwrap(), 4);
    });
}

#[test]
fn test_math_sqrt() {
    test_math_expr("Math.Sqrt(4.0)", |r: Value| {
        assert_eq!(r.as_float().unwrap(), 2.0);
    });

    test_math_expr("Math.Sqrt(-1.0)", |r: Value| {
        assert!(r.as_float().unwrap().is_nan());
    });
}

#[test]
fn test_math_pow() {
    test_math_expr("Math.Pow(2.0, 3.0)", |r: Value| {
        assert_eq!(r.as_float().unwrap(), 8.0);
    });

    test_math_expr("Math.Pow(5.0, 0.0)", |r: Value| {
        assert_eq!(r.as_float().unwrap(), 1.0);
    });
}

#[test]
fn test_math_trig() {
    test_math_expr("Math.Sin(0.0)", |r: Value| {
        assert!((r.as_float().unwrap() - 0.0).abs() < 1e-10);
    });

    test_math_expr("Math.Sin(Math.PI / 2.0)", |r: Value| {
        assert!((r.as_float().unwrap() - 1.0).abs() < 1e-10);
    });

    test_math_expr("Math.Cos(0.0)", |r: Value| {
        assert!((r.as_float().unwrap() - 1.0).abs() < 1e-10);
    });

    test_math_expr("Math.Cos(Math.PI)", |r: Value| {
        assert!((r.as_float().unwrap() + 1.0).abs() < 1e-10);
    });
}

#[test]
fn test_math_inverse_trig() {
    test_math_expr("Math.Asin(0.0)", |r: Value| {
        assert!((r.as_float().unwrap() - 0.0).abs() < 1e-10);
    });

    test_math_expr("Math.Acos(1.0)", |r: Value| {
        assert!((r.as_float().unwrap() - 0.0).abs() < 1e-10);
    });

    test_math_expr("Math.Atan2(1.0, 0.0)", |r: Value| {
        let pi = core::f64::consts::PI;
        assert!((r.as_float().unwrap() - pi / 2.0).abs() < 1e-10);
    });
}

#[test]
fn test_math_log() {
    test_math_expr("Math.Log(Math.E)", |r: Value| {
        assert!((r.as_float().unwrap() - 1.0).abs() < 1e-10);
    });

    test_math_expr("Math.Log10(100.0)", |r: Value| {
        assert!((r.as_float().unwrap() - 2.0).abs() < 1e-10);
    });

    test_math_expr("Math.Exp(1.0)", |r: Value| {
        let e = core::f64::consts::E;
        assert!((r.as_float().unwrap() - e).abs() < 1e-10);
    });
}

#[test]
fn test_math_edge_cases() {
    test_math_expr("Math.Abs(Math.NAN)", |r: Value| {
        assert!(r.as_float().unwrap().is_nan());
    });

    test_math_expr("Math.Abs(Math.INFINITY)", |r: Value| {
        assert!(r.as_float().unwrap().is_infinite());
        assert!(r.as_float().unwrap().is_sign_positive());
    });

    test_math_expr("Math.Sqrt(Math.INFINITY)", |r: Value| {
        assert!(r.as_float().unwrap().is_infinite());
    });
}

#[test]
fn test_math_composition() {
    test_math_expr(
        "Math.Sqrt(Math.Pow(3.0, 2.0) + Math.Pow(4.0, 2.0))",
        |r: Value| {
            assert!((r.as_float().unwrap() - 5.0).abs() < 1e-10);
        },
    );

    test_math_expr("Math.Sin(Math.PI / 6.0)", |r: Value| {
        assert!((r.as_float().unwrap() - 0.5).abs() < 1e-10);
    });
}
