use bumpalo::Bump;

use crate::evaluator::ExecutionErrorKind;
use crate::values::RawValue;

/// A generic adapter for VM operations that need type information at runtime.
///
/// This trait provides a unified interface for operations like function calls,
/// type casts, and other operations that can't be implemented purely with
/// untyped `RawValue`s.
pub trait GenericAdapter {
    /// Number of arguments this adapter expects from the stack.
    fn num_args(&self) -> usize;

    /// Execute the operation with arguments from the stack.
    ///
    /// For `FunctionAdapter`: args includes the function as the last element.
    /// For `CastAdapter`: args contains exactly one element (the value to cast).
    fn call(&self, arena: &Bump, args: &[RawValue]) -> Result<RawValue, ExecutionErrorKind>;

    /// Return a human-readable description of this adapter for debugging.
    ///
    /// Example: "ArrayContains(Str, Array[Str])" or "Cast(Int -> Float)"
    fn name(&self) -> alloc::string::String;
}
