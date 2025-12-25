//! Math Package
//!
//! Provides mathematical functions and constants for Melbi.
//!
//! Constants: PI, E, TAU, INFINITY, NAN
//! Functions: Abs, Min, Max, Clamp, Floor, Ceil, Round, Sqrt, Pow,
//!            Sin, Cos, Tan, Asin, Acos, Atan, Atan2, Log, Log10, Exp

use crate::{types::manager::TypeManager, values::dynamic::Value};
use melbi_macros::{melbi_const, melbi_fn, melbi_package};

#[melbi_package]
pub mod math {
    use super::*;

    // ========================================================================
    // Constants
    // ========================================================================

    /// The mathematical constant π (pi)
    #[melbi_const(name = PI)]
    fn const_pi<'a>(
        _arena: &'a bumpalo::Bump,
        type_mgr: &'a TypeManager<'a>,
    ) -> Value<'a, 'a> {
        Value::float(type_mgr, core::f64::consts::PI)
    }

    /// Euler's number e
    #[melbi_const(name = E)]
    fn const_e<'a>(
        _arena: &'a bumpalo::Bump,
        type_mgr: &'a TypeManager<'a>,
    ) -> Value<'a, 'a> {
        Value::float(type_mgr, core::f64::consts::E)
    }

    /// The mathematical constant τ (tau = 2π)
    #[melbi_const(name = TAU)]
    fn const_tau<'a>(
        _arena: &'a bumpalo::Bump,
        type_mgr: &'a TypeManager<'a>,
    ) -> Value<'a, 'a> {
        Value::float(type_mgr, core::f64::consts::TAU)
    }

    /// Positive infinity
    #[melbi_const(name = INFINITY)]
    fn const_infinity<'a>(
        _arena: &'a bumpalo::Bump,
        type_mgr: &'a TypeManager<'a>,
    ) -> Value<'a, 'a> {
        Value::float(type_mgr, f64::INFINITY)
    }

    /// Not a Number (NaN)
    #[melbi_const(name = NAN)]
    fn const_nan<'a>(
        _arena: &'a bumpalo::Bump,
        type_mgr: &'a TypeManager<'a>,
    ) -> Value<'a, 'a> {
        Value::float(type_mgr, f64::NAN)
    }

    // ========================================================================
    // Basic Operations
    // ========================================================================

    /// Absolute value of a float
    #[melbi_fn(name = Abs)]
    fn math_abs(value: f64) -> f64 {
        value.abs()
    }

    /// Minimum of two floats
    #[melbi_fn(name = Min)]
    fn math_min(a: f64, b: f64) -> f64 {
        a.min(b)
    }

    /// Maximum of two floats
    #[melbi_fn(name = Max)]
    fn math_max(a: f64, b: f64) -> f64 {
        a.max(b)
    }

    /// Clamp a value between min and max
    #[melbi_fn(name = Clamp)]
    fn math_clamp(value: f64, min: f64, max: f64) -> f64 {
        value.clamp(min, max)
    }

    // ========================================================================
    // Rounding Functions
    // ========================================================================

    /// Floor function - returns largest integer <= x
    #[melbi_fn(name = Floor)]
    fn math_floor(value: f64) -> i64 {
        value.floor() as i64
    }

    /// Ceiling function - returns smallest integer >= x
    #[melbi_fn(name = Ceil)]
    fn math_ceil(value: f64) -> i64 {
        value.ceil() as i64
    }

    /// Round to nearest integer
    #[melbi_fn(name = Round)]
    fn math_round(value: f64) -> i64 {
        value.round() as i64
    }

    // ========================================================================
    // Exponentiation
    // ========================================================================

    /// Square root
    #[melbi_fn(name = Sqrt)]
    fn math_sqrt(value: f64) -> f64 {
        // Note: sqrt of negative returns NaN (IEEE 754 semantics)
        value.sqrt()
    }

    /// Power function - base^exp
    #[melbi_fn(name = Pow)]
    fn math_pow(base: f64, exp: f64) -> f64 {
        base.powf(exp)
    }

    // ========================================================================
    // Trigonometry
    // ========================================================================

    /// Sine function
    #[melbi_fn(name = Sin)]
    fn math_sin(value: f64) -> f64 {
        value.sin()
    }

    /// Cosine function
    #[melbi_fn(name = Cos)]
    fn math_cos(value: f64) -> f64 {
        value.cos()
    }

    /// Tangent function
    #[melbi_fn(name = Tan)]
    fn math_tan(value: f64) -> f64 {
        value.tan()
    }

    /// Arc sine function
    #[melbi_fn(name = Asin)]
    fn math_asin(value: f64) -> f64 {
        value.asin()
    }

    /// Arc cosine function
    #[melbi_fn(name = Acos)]
    fn math_acos(value: f64) -> f64 {
        value.acos()
    }

    /// Arc tangent function
    #[melbi_fn(name = Atan)]
    fn math_atan(value: f64) -> f64 {
        value.atan()
    }

    /// Two-argument arc tangent function
    #[melbi_fn(name = Atan2)]
    fn math_atan2(y: f64, x: f64) -> f64 {
        y.atan2(x)
    }

    // ========================================================================
    // Logarithms
    // ========================================================================

    /// Natural logarithm (base e)
    #[melbi_fn(name = Log)]
    fn math_log(value: f64) -> f64 {
        value.ln()
    }

    /// Base-10 logarithm
    #[melbi_fn(name = Log10)]
    fn math_log10(value: f64) -> f64 {
        value.log10()
    }

    /// Exponential function (e^x)
    #[melbi_fn(name = Exp)]
    fn math_exp(value: f64) -> f64 {
        value.exp()
    }
}

// Re-export registration functions for cleaner access
pub use math::{register_math_functions, register_math_package};

#[cfg(test)]
#[path = "math_test.rs"]
mod math_test;
