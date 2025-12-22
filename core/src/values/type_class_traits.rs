//! Rust traits corresponding to Melbi type classes.
//!
//! These traits enable generic FFI functions to use trait bounds that correspond
//! to Melbi's type class system. For example, a Rust function can be generic over
//! `T: Numeric` and the `#[melbi_fn]` macro will generate dispatch code for all
//! types that implement the Melbi Numeric type class (Int and Float).
//!
//! See also: `crate::types::type_class::TypeClassId` for the type-level representation.

use core::ops::{Add, Div, Mul, Sub};

use crate::values::typed::Bridge;

/// Base trait for all Melbi-compatible types.
///
/// This trait is the root of the Melbi type class hierarchy. All types that can
/// be used in generic FFI functions must implement `Melbi`. It provides the
/// fundamental capabilities needed for FFI bridging:
///
/// - `Copy`: Values can be passed by value efficiently
/// - `Bridge`: Values can be converted to/from Melbi's runtime representation
///
/// # Type Class Hierarchy
///
/// ```text
/// Melbi (base trait)
/// ├── Numeric (arithmetic: +, -, *, /)
/// ├── Ord (comparison: <, >, <=, >=)    [future]
/// ├── Hashable (map keys)               [future]
/// ├── Containable (needle in haystack)  [future]
/// └── Indexable (container access)      [future]
/// ```
///
/// # Usage
///
/// For unconstrained generic parameters (parametric polymorphism), use `Melbi`:
///
/// ```ignore
/// #[melbi_fn]
/// fn identity<T: Melbi>(x: T) -> T { x }
/// ```
///
/// For constrained generic parameters, use specific type classes:
///
/// ```ignore
/// #[melbi_fn]
/// fn square<T: Numeric>(x: T) -> T { x * x }
/// ```
pub trait Melbi: Copy + Bridge {}

impl Melbi for i64 {}
impl Melbi for f64 {}

/// Numeric trait for arithmetic operations.
///
/// Corresponds to Melbi's Numeric type class, which includes Int and Float.
/// This trait is implemented for `i64` (Int) and `f64` (Float).
///
/// # Example
///
/// ```ignore
/// use melbi_macros::melbi_fn;
/// use melbi_core::values::Numeric;
///
/// #[melbi_fn]
/// fn square<T: Numeric>(x: T) -> T {
///     x * x
/// }
/// ```
pub trait Numeric:
    Melbi + Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Div<Output = Self>
{
}

impl Numeric for i64 {}
impl Numeric for f64 {}
