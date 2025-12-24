//! Melbi - A flexible, embeddable expression language
//!
//! # Overview
//!
//! Melbi is an expression-focused scripting language designed for safe evaluation
//! of user-defined logic in host applications. Common use cases include:
//!
//! - Email filters and routing rules
//! - Feature flags and conditional logic
//! - Data transformations and mappings
//! - Business rules engines
//!
//! # Quick Start
//!
//! ```
//! use melbi::{Engine, EngineOptions};
//! use melbi::values::{binder::Binder, dynamic::Value};
//! use bumpalo::Bump;
//!
//! // Create an arena for type and environment data
//! let arena = Bump::new();
//! let options = EngineOptions::default();
//!
//! // Create an engine with a global environment
//! let engine = Engine::new(options, &arena, |_arena, type_mgr, env| {
//!     // Register a constant
//!     env.bind("PI", Value::float(type_mgr, std::f64::consts::PI))
//! });
//!
//! // Compile an expression
//! let expr = engine.compile(Default::default(), "PI * 2.0", &[]).unwrap();
//!
//! // Execute in a separate arena
//! let val_arena = Bump::new();
//! let result = expr.run(Default::default(), &val_arena, &[]).unwrap();
//! let result_float = result.as_float().unwrap();
//! assert!((result_float - (std::f64::consts::PI * 2.0)).abs() < 0.0001);
//! ```
//!
//! # API Tiers
//!
//! Melbi provides two API tiers:
//!
//! 1. **Dynamic API** (`run`): Runtime validation, works from any language
//! 2. **Unchecked API** (`run_unchecked`): No validation, maximum performance
//!
//! # FFI Support
//!
//! Register native Rust functions using the `NativeFunction` wrapper:
//!
//! ```
//! use melbi::{Engine, EngineOptions, ExecutionError};
//! use melbi::values::{binder::Binder, FfiContext, NativeFunction, dynamic::Value};
//! use bumpalo::Bump;
//!
//! fn add<'types, 'arena>(
//!     ctx: &FfiContext<'types, 'arena>,
//!     args: &[Value<'types, 'arena>],
//! ) -> Result<Value<'types, 'arena>, ExecutionError> {
//!     debug_assert!(args.len() == 2);
//!     let a = args[0].as_int().expect("arg should be int");
//!     let b = args[1].as_int().expect("arg should be int");
//!     Ok(Value::int(ctx.type_mgr(), a + b))
//! }
//!
//! let arena = Bump::new();
//! let options = EngineOptions::default();
//! let engine = Engine::new(options, &arena, |arena, type_mgr, env| {
//!     let add_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());
//!     env.bind("add", Value::function(arena, NativeFunction::new(add_ty, add)).unwrap())
//! });
//!
//! // Use the function
//! let expr = engine.compile(Default::default(), "add(40, 2)", &[]).unwrap();
//! let val_arena = Bump::new();
//! let result = expr.run(Default::default(), &val_arena, &[]).unwrap();
//! assert_eq!(result.as_int().unwrap(), 42);
//! ```

// Error rendering utilities
pub mod error_renderer;
pub use error_renderer::{RenderConfig, render_error, render_error_to};

// Re-export public API from melbi_core
pub use melbi_core::api::{
    CompileOptions, CompileOptionsOverride, CompiledExpression, Diagnostic, Engine, EngineOptions,
    EnvironmentBuilder, Error, RelatedInfo, RunOptions, RunOptionsOverride, Severity,
};

// Re-export commonly used types and values
pub use melbi_core::types::{
    self, Type,
    manager::TypeManager,
    traits::{TypeBuilder, TypeView},
};
pub use melbi_core::values::{self, Function, NativeFn, NativeFunction, dynamic::Value};

// Re-export errors
// TODO: Currently this is user facing only because FFI functions need
// to return this same error type. Should we have a separate error type for FFI?
pub use melbi_core::evaluator::ExecutionError;
