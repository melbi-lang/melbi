//! Math Package
//!
//! Provides mathematical functions and constants for Melbi.
//!
//! Constants: PI, E, TAU, INFINITY, NAN
//! Functions: Abs, Min, Max, Clamp, Floor, Ceil, Round, Sqrt, Pow,
//!            Sin, Cos, Tan, Asin, Acos, Atan, Atan2, Log, Log10, Exp

use crate::{
    types::manager::TypeManager,
    values::{dynamic::Value, from_raw::TypeError},
};
use bumpalo::Bump;
use melbi_macros::melbi_fn;

// ============================================================================
// Basic Operations
// ============================================================================

/// Absolute value of a float
#[melbi_fn(name = "Abs")]
fn math_abs(value: f64) -> f64 {
    value.abs()
}

/// Minimum of two floats
#[melbi_fn(name = "Min")]
fn math_min(a: f64, b: f64) -> f64 {
    a.min(b)
}

/// Maximum of two floats
#[melbi_fn(name = "Max")]
fn math_max(a: f64, b: f64) -> f64 {
    a.max(b)
}

/// Clamp a value between min and max
#[melbi_fn(name = "Clamp")]
fn math_clamp(value: f64, min: f64, max: f64) -> f64 {
    value.clamp(min, max)
}

// ============================================================================
// Rounding Functions
// ============================================================================

/// Floor function - returns largest integer <= x
#[melbi_fn(name = "Floor")]
fn math_floor(value: f64) -> i64 {
    value.floor() as i64
}

/// Ceiling function - returns smallest integer >= x
#[melbi_fn(name = "Ceil")]
fn math_ceil(value: f64) -> i64 {
    value.ceil() as i64
}

/// Round to nearest integer
#[melbi_fn(name = "Round")]
fn math_round(value: f64) -> i64 {
    value.round() as i64
}

// ============================================================================
// Exponentiation
// ============================================================================

/// Square root
#[melbi_fn(name = "Sqrt")]
fn math_sqrt(value: f64) -> f64 {
    // Note: sqrt of negative returns NaN (IEEE 754 semantics)
    value.sqrt()
}

/// Power function - base^exp
#[melbi_fn(name = "Pow")]
fn math_pow(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

// ============================================================================
// Trigonometry
// ============================================================================

/// Sine function
#[melbi_fn(name = "Sin")]
fn math_sin(value: f64) -> f64 {
    value.sin()
}

/// Cosine function
#[melbi_fn(name = "Cos")]
fn math_cos(value: f64) -> f64 {
    value.cos()
}

/// Tangent function
#[melbi_fn(name = "Tan")]
fn math_tan(value: f64) -> f64 {
    value.tan()
}

/// Arc sine function
#[melbi_fn(name = "Asin")]
fn math_asin(value: f64) -> f64 {
    value.asin()
}

/// Arc cosine function
#[melbi_fn(name = "Acos")]
fn math_acos(value: f64) -> f64 {
    value.acos()
}

/// Arc tangent function
#[melbi_fn(name = "Atan")]
fn math_atan(value: f64) -> f64 {
    value.atan()
}

/// Two-argument arc tangent function
#[melbi_fn(name = "Atan2")]
fn math_atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

// ============================================================================
// Logarithms
// ============================================================================

/// Natural logarithm (base e)
#[melbi_fn(name = "Log")]
fn math_log(value: f64) -> f64 {
    value.ln()
}

/// Base-10 logarithm
#[melbi_fn(name = "Log10")]
fn math_log10(value: f64) -> f64 {
    value.log10()
}

/// Exponential function (e^x)
#[melbi_fn(name = "Exp")]
fn math_exp(value: f64) -> f64 {
    value.exp()
}

// ============================================================================
// Package Builder
// ============================================================================

/// Build the Math package as a record containing all math functions and constants.
///
/// The package includes:
/// - Constants: PI, E, TAU, INFINITY, NAN
/// - Basic operations: Abs, Min, Max, Clamp
/// - Rounding: Floor, Ceil, Round
/// - Exponentiation: Sqrt, Pow
/// - Trigonometry: Sin, Cos, Tan, Asin, Acos, Atan, Atan2
/// - Logarithms: Log, Log10, Exp
///
/// # Example
///
/// ```ignore
/// let math = build_math_package(arena, type_mgr)?;
/// env.register("Math", math)?;
/// ```
pub fn build_math_package<'arena>(
    arena: &'arena Bump,
    type_mgr: &'arena TypeManager<'arena>,
) -> Result<Value<'arena, 'arena>, TypeError> {
    use crate::values::function::AnnotatedFunction;

    let mut builder = Value::record_builder(type_mgr);

    // Constants
    builder = builder.field("PI", Value::float(type_mgr, core::f64::consts::PI));
    builder = builder.field("E", Value::float(type_mgr, core::f64::consts::E));
    builder = builder.field("TAU", Value::float(type_mgr, core::f64::consts::TAU));
    builder = builder.field("INFINITY", Value::float(type_mgr, f64::INFINITY));
    builder = builder.field("NAN", Value::float(type_mgr, f64::NAN));

    // Basic operations
    builder = Abs::new(type_mgr).register(arena, builder)?;
    builder = Min::new(type_mgr).register(arena, builder)?;
    builder = Max::new(type_mgr).register(arena, builder)?;
    builder = Clamp::new(type_mgr).register(arena, builder)?;

    // Rounding functions
    builder = Floor::new(type_mgr).register(arena, builder)?;
    builder = Ceil::new(type_mgr).register(arena, builder)?;
    builder = Round::new(type_mgr).register(arena, builder)?;

    // Exponentiation
    builder = Sqrt::new(type_mgr).register(arena, builder)?;
    builder = Pow::new(type_mgr).register(arena, builder)?;

    // Trigonometry
    builder = Sin::new(type_mgr).register(arena, builder)?;
    builder = Cos::new(type_mgr).register(arena, builder)?;
    builder = Tan::new(type_mgr).register(arena, builder)?;
    builder = Asin::new(type_mgr).register(arena, builder)?;
    builder = Acos::new(type_mgr).register(arena, builder)?;
    builder = Atan::new(type_mgr).register(arena, builder)?;
    builder = Atan2::new(type_mgr).register(arena, builder)?;

    // Logarithms
    builder = Log::new(type_mgr).register(arena, builder)?;
    builder = Log10::new(type_mgr).register(arena, builder)?;
    builder = Exp::new(type_mgr).register(arena, builder)?;

    builder.build(arena)
}

#[cfg(test)]
#[path = "math_test.rs"]
mod math_test;
