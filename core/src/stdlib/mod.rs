//! Melbi Standard Library
//!
//! This module provides the standard library packages for Melbi, including:
//! - Int: Integer arithmetic operations (Quot, Rem, Div, Mod)
//! - Math: Mathematical functions and constants
//! - String: String manipulation functions
//! - Array: Array operations
//! - Option: Option utilities (future)
//!
//! Each package is implemented as a record containing functions and constants.
//! Packages are built using native Rust functions (FFI) and registered in the
//! global environment before user code executes.

use crate::types::manager::TypeManager;
use crate::values::binder::Binder;
use bumpalo::Bump;

pub mod array;
pub mod int;
pub mod math;
pub mod string;

// Re-export for convenience
pub use array::{register_array_functions, register_array_package};
pub use int::{register_int_functions, register_int_package};
pub use math::{register_math_functions, register_math_package};
pub use string::{register_string_functions, register_string_package};

/// Register all standard library packages in the environment.
///
/// This is a convenience function that registers all "default" standard library
/// packages (Math, String, Array, Int) in the global environment. Use this in your
/// Engine initialization to get the full standard library.
///
/// # Example
///
/// ```ignore
/// let engine = Engine::new(options, &arena, |arena, type_mgr, env| {
///     register_stdlib(arena, type_mgr, env)
/// });
/// ```
///
/// If you want more control over which packages to include, you can register
/// them individually using `register_math_package()`, `register_string_package()`, etc.
///
/// # Panics
///
/// Panics if there are duplicate bindings within a package. This indicates a bug
/// in the stdlib implementation (e.g., two functions with the same name).
pub fn register_stdlib<'a, B>(
    arena: &'a Bump,
    type_mgr: &'a TypeManager<'a>,
    env: B,
) -> B
where
    B: Binder<'a, 'a>,
{
    let env = register_math_package(arena, type_mgr, env);
    let env = register_string_package(arena, type_mgr, env);
    let env = register_array_package(arena, type_mgr, env);
    let env = register_int_package(arena, type_mgr, env);

    // Future packages will be added here:
    // - Option package
    // - etc.

    env
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{CompileOptionsOverride, Engine, EngineOptions};

    #[test]
    fn test_register_stdlib() {
        // Test that register_stdlib successfully registers all packages
        let options = EngineOptions::default();
        let arena = Bump::new();

        let engine = Engine::new(options, &arena, register_stdlib);

        let compile_opts = CompileOptionsOverride::default();

        // Test Math package is available
        let math_expr = engine
            .compile(compile_opts, "Math.PI", &[])
            .expect("Math.PI should compile");
        let val_arena = Bump::new();
        let result = math_expr
            .run(Default::default(), &val_arena, &[])
            .expect("Math.PI should execute");
        let pi = result.as_float().unwrap();
        assert!((pi - std::f64::consts::PI).abs() < 1e-10);

        // Test String package is available
        let string_expr = engine
            .compile(compile_opts, "String.Len(\"hello\")", &[])
            .expect("String.Len should compile");
        let val_arena2 = Bump::new();
        let result = string_expr
            .run(Default::default(), &val_arena2, &[])
            .expect("String.Len should execute");
        assert_eq!(result.as_int().unwrap(), 5);
    }

    #[test]
    fn test_register_stdlib_both_packages_work_together() {
        // Test that both packages can be used in the same expression
        let options = EngineOptions::default();
        let arena = Bump::new();

        let engine = Engine::new(options, &arena, register_stdlib);

        let compile_opts = CompileOptionsOverride::default();
        let expr = engine
            .compile(compile_opts, "String.Len(f\"{Math.Floor(Math.PI)}\")", &[])
            .expect("expression should compile");

        let val_arena = Bump::new();
        let result = expr
            .run(Default::default(), &val_arena, &[])
            .expect("expression should execute");
        // Math.Floor(Math.PI) = 3.0, stringified = "3", length = 1
        assert_eq!(result.as_int().unwrap(), 1);
    }
}
