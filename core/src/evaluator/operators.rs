//! Binary and unary operator implementations.

use crate::evaluator::ExecutionErrorKind;
use crate::evaluator::RuntimeError::*;
use crate::parser::{BinaryOp, ComparisonOp, UnaryOp};

/// Evaluate a binary operation on two integers.
///
/// Uses wrapping arithmetic to prevent panics on overflow.
/// Division by zero returns an error.
pub(super) fn eval_binary_int(
    op: BinaryOp,
    left: i64,
    right: i64,
) -> Result<i64, ExecutionErrorKind> {
    match op {
        BinaryOp::Add => Ok(left.wrapping_add(right)),
        BinaryOp::Sub => Ok(left.wrapping_sub(right)),
        BinaryOp::Mul => Ok(left.wrapping_mul(right)),
        BinaryOp::Div => {
            if right == 0 {
                Err(DivisionByZero {}.into())
            } else if left == i64::MIN && right == -1 {
                Err(IntegerOverflow {}.into())
            } else {
                Ok(left.div_euclid(right))
            }
        }
        BinaryOp::Pow => {
            // Handle power specially to avoid overflow panics
            if right < 0 {
                // Negative exponents for integers result in 0 (floor division semantics)
                Ok(0)
            } else if right > u32::MAX as i64 {
                // Exponent too large, will overflow or underflow
                // Return 0 for simplicity (matches negative exponent behavior)
                Ok(0)
            } else {
                // Use wrapping_pow for safe exponentiation
                Ok(left.wrapping_pow(right as u32))
            }
        }
    }
}

/// Evaluate a binary operation on two floats.
///
/// Follows IEEE 754 semantics (produces inf/nan rather than panicking).
pub(super) fn eval_binary_float(op: BinaryOp, left: f64, right: f64) -> f64 {
    match op {
        BinaryOp::Add => left + right,
        BinaryOp::Sub => left - right,
        BinaryOp::Mul => left * right,
        BinaryOp::Div => left / right, // Division by zero produces inf
        BinaryOp::Pow => left.powf(right),
    }
}

/// Evaluate a unary operation on an integer.
///
/// Uses wrapping arithmetic for negation to prevent panics on overflow.
pub(super) fn eval_unary_int(op: UnaryOp, value: i64) -> i64 {
    match op {
        UnaryOp::Neg => value.wrapping_neg(),
        UnaryOp::Not => {
            // Type checker should have caught this
            unreachable!("Not operator not valid for integer")
        }
    }
}

/// Evaluate a unary operation on a float.
pub(super) fn eval_unary_float(op: UnaryOp, value: f64) -> f64 {
    match op {
        UnaryOp::Neg => -value,
        UnaryOp::Not => {
            // Type checker should have caught this
            unreachable!("Not operator not valid for float")
        }
    }
}

/// Evaluate a unary operation on a boolean.
pub(super) fn eval_unary_bool(op: UnaryOp, value: bool) -> bool {
    match op {
        UnaryOp::Not => !value,
        UnaryOp::Neg => {
            // Type checker should have caught this
            unreachable!("Neg operator not valid for boolean")
        }
    }
}

/// Evaluate a comparison operation on two integers.
pub(super) fn eval_comparison_int(op: ComparisonOp, left: i64, right: i64) -> bool {
    match op {
        ComparisonOp::Eq => left == right,
        ComparisonOp::Neq => left != right,
        ComparisonOp::Lt => left < right,
        ComparisonOp::Gt => left > right,
        ComparisonOp::Le => left <= right,
        ComparisonOp::Ge => left >= right,
        ComparisonOp::In | ComparisonOp::NotIn => {
            unreachable!("In/NotIn not valid for integers")
        }
    }
}

/// Evaluate a comparison operation on two floats.
pub(super) fn eval_comparison_float(op: ComparisonOp, left: f64, right: f64) -> bool {
    match op {
        ComparisonOp::Eq => left == right,
        ComparisonOp::Neq => left != right,
        ComparisonOp::Lt => left < right,
        ComparisonOp::Gt => left > right,
        ComparisonOp::Le => left <= right,
        ComparisonOp::Ge => left >= right,
        ComparisonOp::In | ComparisonOp::NotIn => {
            unreachable!("In/NotIn not valid for floats")
        }
    }
}

/// Evaluate a comparison operation on two booleans.
pub(super) fn eval_comparison_bool(op: ComparisonOp, left: bool, right: bool) -> bool {
    match op {
        ComparisonOp::Eq => left == right,
        ComparisonOp::Neq => left != right,
        ComparisonOp::Lt | ComparisonOp::Gt | ComparisonOp::Le | ComparisonOp::Ge => {
            // Type checker should have caught this
            unreachable!("Ordering comparison on Bool in type-checked expression")
        }
        ComparisonOp::In | ComparisonOp::NotIn => {
            unreachable!("In/NotIn not valid for booleans")
        }
    }
}

/// Evaluate a comparison operation on two strings.
pub(super) fn eval_comparison_string(op: ComparisonOp, left: &str, right: &str) -> bool {
    match op {
        ComparisonOp::Eq => left == right,
        ComparisonOp::Neq => left != right,
        ComparisonOp::Lt => left < right,
        ComparisonOp::Gt => left > right,
        ComparisonOp::Le => left <= right,
        ComparisonOp::Ge => left >= right,
        ComparisonOp::In | ComparisonOp::NotIn => eval_containment_string(op, left, right),
    }
}

/// Evaluate a comparison operation on two byte slices.
pub(super) fn eval_comparison_bytes(op: ComparisonOp, left: &[u8], right: &[u8]) -> bool {
    match op {
        ComparisonOp::Eq => left == right,
        ComparisonOp::Neq => left != right,
        ComparisonOp::Lt => left < right,
        ComparisonOp::Gt => left > right,
        ComparisonOp::Le => left <= right,
        ComparisonOp::Ge => left >= right,
        ComparisonOp::In | ComparisonOp::NotIn => eval_containment_bytes(op, left, right),
    }
}

/// Evaluate a containment operation on strings (substring check).
///
/// Returns true if the needle (left) is found within the haystack (right).
fn eval_containment_string(op: ComparisonOp, needle: &str, haystack: &str) -> bool {
    let result = haystack.contains(needle);
    match op {
        ComparisonOp::In => result,
        ComparisonOp::NotIn => !result,
        _ => unreachable!("eval_containment_string called with non-containment operator"),
    }
}

/// Evaluate a containment operation on byte slices (byte sequence check).
///
/// Returns true if the needle (left) is found within the haystack (right).
fn eval_containment_bytes(op: ComparisonOp, needle: &[u8], haystack: &[u8]) -> bool {
    let result = if needle.is_empty() {
        // Empty needle is always contained
        true
    } else if needle.len() > haystack.len() {
        // Needle longer than haystack can't be contained
        false
    } else {
        // Search for needle in haystack using sliding windows
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    };

    match op {
        ComparisonOp::In => result,
        ComparisonOp::NotIn => !result,
        _ => unreachable!("eval_containment_bytes called with non-containment operator"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evaluator::RuntimeError;

    #[test]
    fn int_add() {
        assert_eq!(eval_binary_int(BinaryOp::Add, 2, 3).unwrap(), 5);
        assert_eq!(eval_binary_int(BinaryOp::Add, -5, 3).unwrap(), -2);
    }

    #[test]
    fn int_sub() {
        assert_eq!(eval_binary_int(BinaryOp::Sub, 10, 4).unwrap(), 6);
        assert_eq!(eval_binary_int(BinaryOp::Sub, 3, 10).unwrap(), -7);
    }

    #[test]
    fn int_mul() {
        assert_eq!(eval_binary_int(BinaryOp::Mul, 3, 4).unwrap(), 12);
        assert_eq!(eval_binary_int(BinaryOp::Mul, -2, 5).unwrap(), -10);
    }

    #[test]
    fn int_div() {
        assert_eq!(eval_binary_int(BinaryOp::Div, 10, 2).unwrap(), 5);
        assert_eq!(eval_binary_int(BinaryOp::Div, 7, 3).unwrap(), 2);
    }

    #[test]
    fn int_div_by_zero() {
        let result = eval_binary_int(BinaryOp::Div, 10, 0);
        assert!(matches!(
            result.as_ref().map(|_| ()),
            Err(crate::evaluator::ExecutionErrorKind::Runtime(
                RuntimeError::DivisionByZero {}
            ))
        ));
    }

    #[test]
    fn int_pow() {
        assert_eq!(eval_binary_int(BinaryOp::Pow, 2, 10).unwrap(), 1024);
        assert_eq!(eval_binary_int(BinaryOp::Pow, 3, 3).unwrap(), 27);
        assert_eq!(eval_binary_int(BinaryOp::Pow, 5, 0).unwrap(), 1);
    }

    #[test]
    fn int_pow_negative_exponent() {
        // Negative exponents for integers return 0 (floor semantics)
        assert_eq!(eval_binary_int(BinaryOp::Pow, 2, -1).unwrap(), 0);
    }

    #[test]
    fn int_wrapping_overflow() {
        // Test that we wrap on overflow rather than panic
        let result = eval_binary_int(BinaryOp::Add, i64::MAX, 1).unwrap();
        assert_eq!(result, i64::MIN);

        let result = eval_binary_int(BinaryOp::Mul, i64::MAX, 2).unwrap();
        assert_eq!(result, -2);
    }

    #[test]
    fn float_add() {
        let result = eval_binary_float(BinaryOp::Add, 3.14, 2.0);
        assert!((result - 5.14).abs() < 0.0001);
    }

    #[test]
    fn float_div() {
        assert_eq!(eval_binary_float(BinaryOp::Div, 10.0, 3.0), 10.0 / 3.0);
    }

    #[test]
    fn float_div_by_zero() {
        // Float division by zero produces infinity (IEEE 754)
        let result = eval_binary_float(BinaryOp::Div, 10.0, 0.0);
        assert!(result.is_infinite() && result.is_sign_positive());
    }

    #[test]
    fn float_pow() {
        assert_eq!(eval_binary_float(BinaryOp::Pow, 2.0, 3.0), 8.0);
    }
}
