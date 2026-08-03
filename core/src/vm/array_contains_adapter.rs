//! Array containment adapter for the VM.
//!
//! This adapter enables the `in` and `not in` operators for arrays in the bytecode VM.
//! Since the VM operates on untyped `RawValue`s, we need type information at runtime
//! to properly compare array elements. The adapter stores the element type and uses
//! it to convert raw values to typed `Value`s for comparison.
//!
//! # Performance
//!
//! Element lookup uses linear search with O(n) time complexity, where n is the array
//! length. For large arrays with frequent containment checks, consider using a `Map`
//! for O(log n) lookups instead.

use bumpalo::Bump;

use crate::evaluator::ExecutionErrorKind;
use crate::parser::ComparisonOp;
use crate::types::Type;
use crate::values::RawValue;
use crate::values::dynamic::Value;
use crate::vm::GenericAdapter;

/// Adapter for array containment operations (`elem in array` / `elem not in array`).
///
/// Stores the element type needed to compare values at runtime. The element type
/// must match both the needle (element being searched for) and the array's element type.
pub struct ArrayContainsAdapter<'t> {
    element_type: &'t Type<'t>,
    op: ComparisonOp,
}

impl<'t> ArrayContainsAdapter<'t> {
    pub fn new(element_type: &'t Type<'t>, op: ComparisonOp) -> Self {
        debug_assert!(matches!(op, ComparisonOp::In | ComparisonOp::NotIn));
        ArrayContainsAdapter { element_type, op }
    }
}

impl<'t> GenericAdapter for ArrayContainsAdapter<'t> {
    fn num_args(&self) -> usize {
        2 // elem and array
    }

    fn call(&self, _arena: &Bump, args: &[RawValue]) -> Result<RawValue, ExecutionErrorKind> {
        let elem_raw = args[0];
        let array_raw = args[1];

        // Convert RawValue to Value using element type
        let needle = Value::from_raw_unchecked(self.element_type, elem_raw);

        // Get array data
        let array = crate::values::raw::ArrayData::from_raw_value(array_raw);

        // Search for the element (linear scan, O(n))
        let found = (0..array.length()).any(|i| {
            // SAFETY: `i` is guaranteed to be in bounds by the range `0..array.length()`.
            let elem_raw = unsafe { array.get_unchecked(i) };
            let elem = Value::from_raw_unchecked(self.element_type, elem_raw);
            elem == needle
        });

        let result = match self.op {
            ComparisonOp::In => found,
            ComparisonOp::NotIn => !found,
            _ => unreachable!("ArrayContainsAdapter only handles In/NotIn"),
        };

        Ok(RawValue::make_bool(result))
    }

    fn name(&self) -> alloc::string::String {
        let op_name = match self.op {
            ComparisonOp::In => "in",
            ComparisonOp::NotIn => "not in",
            _ => "?",
        };
        alloc::format!(
            "ArrayContains({} {} Array[{}])",
            self.element_type,
            op_name,
            self.element_type
        )
    }
}
