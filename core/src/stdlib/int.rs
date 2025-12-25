//! Int Package
//!
//! Provides integer division and modulus operations for Melbi.
//!
//! Functions:
//! - `Quot(a, b)`: Truncated division (rounds towards zero)
//! - `Rem(a, b)`: Remainder of truncated division (sign matches dividend)
//! - `Div(a, b)`: Euclidean division (remainder always non-negative)
//! - `Mod(a, b)`: Euclidean modulus (always non-negative)

use crate::evaluator::RuntimeError;
use melbi_macros::{melbi_fn, melbi_package};

// ============================================================================
// Error Handling
// ============================================================================

/// Check for division by zero and return an appropriate error
#[inline]
fn check_division_by_zero(b: i64) -> Result<(), RuntimeError> {
    if b == 0 {
        Err(RuntimeError::DivisionByZero {})
    } else {
        Ok(())
    }
}

/// Check for overflow in i64::MIN / -1 case and return an appropriate error
#[inline]
fn check_overflow(a: i64, b: i64) -> Result<(), RuntimeError> {
    if a == i64::MIN && b == -1 {
        Err(RuntimeError::IntegerOverflow {})
    } else {
        Ok(())
    }
}

#[melbi_package]
pub mod int {
    use super::*;

    // ========================================================================
    // Truncated Division (C-style)
    // ========================================================================

    /// Performs truncated division (rounding towards zero).
    ///
    /// This is the standard division behavior in C, Java, and most CPU hardware.
    /// The result is equal to `a / b` with the fractional part discarded.
    ///
    /// Invariant: `a == (Int.Quot(a, b) * b) + Int.Rem(a, b)`
    ///
    /// Errors:
    /// - DivisionByZero if `b == 0`
    /// - IntegerOverflow if `a == i64::MIN && b == -1`
    ///
    /// Examples:
    /// - `Int.Quot(-7, 3)  -> -2`
    /// - `Int.Quot(7, -3)  -> -2`
    /// - `Int.Quot(-7, -3) ->  2`
    #[melbi_fn(name = Quot)]
    fn int_quot(a: i64, b: i64) -> Result<i64, RuntimeError> {
        check_division_by_zero(b)?;
        check_overflow(a, b)?;
        Ok(a / b)
    }

    /// Returns the remainder of truncated division.
    ///
    /// The sign of the result always matches the sign of the dividend `a`.
    /// This is generally not useful for cyclic logic (like wrapping arrays),
    /// but is useful for digit extraction or hardware emulation.
    ///
    /// Errors:
    /// - DivisionByZero if `b == 0`
    /// - IntegerOverflow if `a == i64::MIN && b == -1`
    ///
    /// Examples:
    /// - `Int.Rem(-7, 3)  -> -1`
    /// - `Int.Rem(7, -3)  ->  1`
    /// - `Int.Rem(-7, -3) -> -1`
    #[melbi_fn(name = Rem)]
    fn int_rem(a: i64, b: i64) -> Result<i64, RuntimeError> {
        check_division_by_zero(b)?;
        check_overflow(a, b)?;
        Ok(a % b)
    }

    // ========================================================================
    // Euclidean Division
    // ========================================================================

    /// Performs Euclidean division.
    ///
    /// Calculates the quotient such that the corresponding remainder (`Int.Mod`)
    /// is always non-negative. If `b` is positive, this behaves identical
    /// to floored division (rounding towards negative infinity).
    ///
    /// This is the recommended division operation for consistent mathematical logic.
    ///
    /// Invariant: `a == (Int.Div(a, b) * b) + Int.Mod(a, b)`
    ///
    /// Errors:
    /// - DivisionByZero if `b == 0`
    /// - IntegerOverflow if `a == i64::MIN && b == -1`
    ///
    /// Examples:
    /// - `Int.Div(-7, 3)  -> -3`
    /// - `Int.Div(7, -3)  -> -2`
    /// - `Int.Div(-7, -3) ->  3`
    #[melbi_fn(name = Div)]
    fn int_div(a: i64, b: i64) -> Result<i64, RuntimeError> {
        check_division_by_zero(b)?;
        check_overflow(a, b)?;
        Ok(a.div_euclid(b))
    }

    /// Returns the Euclidean modulus.
    ///
    /// The result is always non-negative (`0 <= result < |b|`), regardless
    /// of the signs of the inputs. This is the correct operator for
    /// array indexing, clock arithmetic, and cyclic wrapping.
    ///
    /// Errors:
    /// - DivisionByZero if `b == 0`
    /// - IntegerOverflow if `a == i64::MIN && b == -1`
    ///
    /// Examples:
    /// - `Int.Mod(-7, 3)  ->  2`
    /// - `Int.Mod(7, -3)  ->  1`
    /// - `Int.Mod(-7, -3) ->  2` (result is positive even if b is negative)
    #[melbi_fn(name = Mod)]
    fn int_mod(a: i64, b: i64) -> Result<i64, RuntimeError> {
        check_division_by_zero(b)?;
        check_overflow(a, b)?;
        Ok(a.rem_euclid(b))
    }
}

// Re-export registration functions for cleaner access
pub use int::{register_int_functions, register_int_package};

#[cfg(test)]
#[path = "int_test.rs"]
mod int_test;
