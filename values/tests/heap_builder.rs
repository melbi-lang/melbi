extern crate alloc;

use alloc::rc::Rc;
use melbi_types::{BoxBuilder, Scalar, Ty, TyKind, ty};
use melbi_values::{
    dynamic::ValueView,
    raw::RawValue,
    traits::{Value, ValueBuilder},
    typed::ArrayView,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct HeapBuilder;

impl ValueBuilder for HeapBuilder {
    type TB = BoxBuilder;

    type Raw = RawValue;

    type ValueHandle = Rc<Value<Self>>;

    type Array = Array;

    fn alloc(&self, value: Value<Self>) -> Self::ValueHandle {
        Rc::new(value)
    }
}

impl ValueView<HeapBuilder> for Value<HeapBuilder> {
    fn ty(&self) -> Ty<BoxBuilder> {
        self.ty().clone()
    }

    fn as_int(&self) -> Option<i64> {
        let TyKind::Scalar(Scalar::Int) = self.ty().kind() else {
            return None;
        };
        Some(self.raw().as_int_unchecked())
    }

    fn as_bool(&self) -> Option<bool> {
        let TyKind::Scalar(Scalar::Bool) = self.ty().kind() else {
            return None;
        };
        Some(self.raw().as_bool_unchecked())
    }

    // Complex Types: Return the associated types from the System
    fn as_array(&self) -> Option<Array> {
        let TyKind::Array(element_type) = self.ty().kind() else {
            return None;
        };
        Some(Array(self.raw().clone(), element_type.clone()))
    }
}

struct Array(RawValue, Ty<BoxBuilder>);

impl ArrayView<Value<HeapBuilder>> for Array {
    fn len(&self) -> usize {
        self.0.as_array_unchecked().len()
    }

    fn get(&self, index: usize) -> Option<Value<HeapBuilder>> {
        let element = self.0.as_array_unchecked().get(index)?;
        Some(Value::new(self.1.clone(), element.clone()))
    }
}

#[test]
fn test_heap_builder_int() {
    let builder = HeapBuilder;
    let tb = BoxBuilder;

    let int_ty = ty!(tb, Int);
    let v = Value::new(int_ty, RawValue::new_int(42)).alloc(&builder);
    let value = v.value();

    assert_eq!(value.as_bool(), None);
    assert_eq!(value.as_int(), Some(42));
}

#[test]
fn test_heap_builder_bool() {
    let builder = HeapBuilder;
    let tb = BoxBuilder;

    let bool_ty = ty!(tb, Bool);
    let v = Value::new(bool_ty, RawValue::new_bool(true)).alloc(&builder);
    let value = v.value();

    assert_eq!(value.as_int(), None);
    assert_eq!(value.as_bool(), Some(true));
}

#[test]
fn test_heap_builder_array() {
    let builder = HeapBuilder;
    let tb = BoxBuilder;

    let int_ty = ty!(tb, Int);
    let array_ty = ty!(tb, Array[Int]);

    let elements = [RawValue::new_int(10), RawValue::new_int(20)];
    let raw_array = RawValue::new_array(&elements);

    let v = Value::new(array_ty, raw_array).alloc(&builder);
    let value = v.value();

    assert_eq!(value.as_int(), None);
    assert_eq!(value.as_bool(), None);

    let array = value.as_array().unwrap();
    assert_eq!(array.len(), 2);

    let el0 = array.get(0).unwrap();
    assert_eq!(el0.ty(), &int_ty);
    assert_eq!(el0.as_int(), Some(10));

    let el1 = array.get(1).unwrap();
    assert_eq!(el1.ty(), &int_ty);
    assert_eq!(el1.as_int(), Some(20));

    assert!(array.get(2).is_none());
}

#[test]
fn test_heap_builder_empty_array() {
    let builder = HeapBuilder;
    let tb = BoxBuilder;

    let array_ty = ty!(tb, Array[Int]);

    let elements = [];
    let raw_array = RawValue::new_array(&elements);

    let v = Value::new(array_ty, raw_array).alloc(&builder);
    let value = v.value();

    let array = value.as_array().unwrap();
    assert_eq!(array.len(), 0);
    assert!(array.is_empty());
    assert!(array.get(0).is_none());
}
