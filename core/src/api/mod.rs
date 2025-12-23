//! Public API for the Melbi expression language.
//!
//! This module provides the stable public API for compiling and executing
//! Melbi expressions. It follows the three-tier design:
//!
//! 1. **Unchecked API**: Maximum performance, no validation (`run_unchecked`)
//! 2. **Dynamic API**: Runtime validation, C FFI compatible (`run`)
//! 3. **Static API**: (Future) Compile-time type checking
//!
//! # Example
//!
//! ```
//! use melbi_core::api::{CompileOptionsOverride, Engine, EngineOptions, RunOptionsOverride};
//! use melbi_core::values::{dynamic::Value, binder::Binder};
//! use bumpalo::Bump;
//!
//! let arena = Bump::new();
//! let options = EngineOptions::default();
//!
//! let engine = Engine::new(options, &arena, |_arena, type_mgr, env| {
//!     // Register constants
//!     env.bind("PI", Value::float(type_mgr, std::f64::consts::PI))
//! });
//!
//! // Compile expression
//! let expr = engine.compile(Default::default(), "PI * 2.0", &[]).unwrap();
//!
//! // Execute
//! let val_arena = Bump::new();
//! let result = expr.run(Default::default(), &val_arena, &[]).unwrap();
//! assert!((result.as_float().unwrap() - 6.28318).abs() < 0.0001);
//! ```

pub mod engine;
pub mod environment;
pub mod error;
pub mod expression;
pub mod options;

pub use engine::Engine;
pub use environment::EnvironmentBuilder;
pub use error::{Diagnostic, Error, RelatedInfo, Severity};
pub use expression::CompiledExpression;
pub use options::{
    CompileOptions, CompileOptionsOverride, EngineOptions, RunOptions, RunOptionsOverride,
};
