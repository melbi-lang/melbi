//! `Array` package for Melbi
use crate::{
    evaluator::ExecutionError,
    types::{
        Type,
        manager::TypeManager,
        traits::{TypeKind, TypeView},
    },
    values::{
        binder::Binder,
        dynamic::Value,
        function::{AnnotatedFunction, FfiContext, Function},
    },
};
use alloc::{vec, vec::Vec};
use bumpalo::Bump;

// ============================================================================
// Basic Functions
// ============================================================================

/// Get the length of an array
fn array_len<'types, 'arena>(
    ctx: &FfiContext<'types, 'arena>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    debug_assert_eq!(args.len(), 1);
    let arr = args[0].as_array().expect("Expected array");
    Ok(Value::int(ctx.type_mgr(), arr.len() as i64))
}

/// Check if an array is empty
fn array_is_empty<'types, 'arena>(
    ctx: &FfiContext<'types, 'arena>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    debug_assert_eq!(args.len(), 1);
    let arr = args[0].as_array().expect("Expected array");
    Ok(Value::bool(ctx.type_mgr(), arr.is_empty()))
}

// ============================================================================
// Slice Functions
// ============================================================================

/// Extract a slice of an array
fn array_slice<'types, 'arena>(
    ctx: &FfiContext<'types, 'arena>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    debug_assert_eq!(args.len(), 3);
    let arr = args[0].as_array().expect("Expected array");
    let start = args[1].as_int().expect("Expected int") as usize;
    let end = args[2].as_int().expect("Expected int") as usize;

    let len = arr.len();
    let start_idx = start.min(len);
    let end_idx = end.min(len);

    let elem_ty = match args[0].ty.view() {
        TypeKind::Array(elem_ty) => elem_ty,
        _ => panic!("Expected array type"),
    };

    if start_idx >= end_idx {
        return Ok(
            Value::array(ctx.arena(), ctx.type_mgr().array(elem_ty), &[])
                .expect("Type error in Array.Slice: empty array construction failed"),
        );
    }

    let slice: Vec<Value<'types, 'arena>> = arr
        .iter()
        .skip(start_idx)
        .take(end_idx - start_idx)
        .collect();
    Ok(
        Value::array(ctx.arena(), ctx.type_mgr().array(elem_ty), &slice)
            .expect("Type error in Array.Slice: array construction failed"),
    )
}

// ============================================================================
// Collection Functions
// ============================================================================

/// Concatenate two arrays
fn array_concat<'types, 'arena>(
    ctx: &FfiContext<'types, 'arena>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    debug_assert_eq!(args.len(), 2);
    let arr1 = args[0].as_array().expect("Expected array");
    let arr2 = args[1].as_array().expect("Expected array");

    let mut result = Vec::new();
    result.extend(arr1.iter());
    result.extend(arr2.iter());

    let elem_ty = match args[0].ty.view() {
        TypeKind::Array(elem_ty) => elem_ty,
        _ => panic!("Expected array type"),
    };

    Ok(
        Value::array(ctx.arena(), ctx.type_mgr().array(elem_ty), &result)
            .expect("Type error in Array.Concat: array construction failed"),
    )
}

/// Flatten an array of arrays
fn array_flatten<'types, 'arena>(
    ctx: &FfiContext<'types, 'arena>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    debug_assert_eq!(args.len(), 1);
    let arr = args[0].as_array().expect("Expected array of arrays");

    let mut result = Vec::new();
    for inner_val in arr.iter() {
        let inner_arr = inner_val.as_array().expect("Expected array element");
        result.extend(inner_arr.iter());
    }

    let inner_elem_ty = match args[0].ty.view() {
        TypeKind::Array(arr_ty) => match arr_ty.view() {
            TypeKind::Array(elem_ty) => elem_ty,
            TypeKind::TypeVar(_) => ctx.type_mgr().fresh_type_var(),
            _ => panic!("Expected array of arrays, got {:?}", arr_ty),
        },
        _ => panic!("Expected array type"),
    };

    Ok(
        Value::array(ctx.arena(), ctx.type_mgr().array(inner_elem_ty), &result)
            .expect("Type error in Array.Flatten: array construction failed"),
    )
}

/// Zip two arrays into an array of tuples (records with fields "first" and "second")
fn array_zip<'types, 'arena>(
    ctx: &FfiContext<'types, 'arena>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    debug_assert_eq!(args.len(), 2);
    let arr1 = args[0].as_array().expect("Expected array");
    let arr2 = args[1].as_array().expect("Expected array");

    let elem_ty1 = match args[0].ty.view() {
        TypeKind::Array(elem_ty) => elem_ty,
        _ => panic!("Expected array type"),
    };
    let elem_ty2 = match args[1].ty.view() {
        TypeKind::Array(elem_ty) => elem_ty,
        _ => panic!("Expected array type"),
    };

    let mut result = Vec::new();
    for (val1, val2) in arr1.iter().zip(arr2.iter()) {
        let tuple = Value::record_builder(ctx.arena(), ctx.type_mgr())
            .bind("first", val1)
            .bind("second", val2)
            .build()
            .expect("Type error in Array.Zip: record construction failed");
        result.push(tuple);
    }

    let tuple_ty = ctx
        .type_mgr()
        .record(vec![("first", elem_ty1), ("second", elem_ty2)]);

    Ok(
        Value::array(ctx.arena(), ctx.type_mgr().array(tuple_ty), &result)
            .expect("Type error in Array.Zip: array construction failed"),
    )
}

// ============================================================================
// Transformation Functions
// ============================================================================

/// Reverse an array
fn array_reverse<'types, 'arena>(
    ctx: &FfiContext<'types, 'arena>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    debug_assert_eq!(args.len(), 1);
    let arr = args[0].as_array().expect("Expected array");

    let mut result: Vec<Value<'types, 'arena>> = arr.iter().collect();
    result.reverse();

    let elem_ty = match args[0].ty.view() {
        TypeKind::Array(elem_ty) => elem_ty,
        _ => panic!("Expected array type"),
    };

    Ok(
        Value::array(ctx.arena(), ctx.type_mgr().array(elem_ty), &result)
            .expect("Type error in Array.Reverse: array construction failed"),
    )
}

// ============================================================================
// Higher-Order Functions
// ============================================================================

/// Map a function over an array
fn array_map<'types, 'arena>(
    ctx: &FfiContext<'types, 'arena>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError> {
    debug_assert_eq!(args.len(), 2);
    let arr = args[0].as_array().expect("Expected array");
    let func = args[1].as_function().expect("Expected function");

    let mut results = Vec::new();
    for elem in arr.iter() {
        let result = unsafe { func.call_unchecked(ctx, &[elem]) }?;
        results.push(result);
    }

    let result_elem_ty = match args[1].ty.view() {
        TypeKind::Function { ret, .. } => ret,
        _ => panic!("Expected function type"),
    };

    Ok(
        Value::array(ctx.arena(), ctx.type_mgr().array(result_elem_ty), &results)
            .expect("Type error in Array.Map: array construction failed"),
    )
}

// ============================================================================
// Package Registration
// ============================================================================

/// Helper struct to wrap a native function pointer and implement the Function trait
struct NativeFunction<'types> {
    name: &'static str,
    ty: &'types Type<'types>,
    ptr: fn(
        &FfiContext<'types, 'types>,
        &[Value<'types, 'types>],
    ) -> Result<Value<'types, 'types>, ExecutionError>,
}

impl<'types> Function<'types, 'types> for NativeFunction<'types> {
    fn ty(&self) -> &'types Type<'types> {
        self.ty
    }

    unsafe fn call_unchecked(
        &self,
        ctx: &FfiContext<'types, 'types>,
        args: &[Value<'types, 'types>],
    ) -> Result<Value<'types, 'types>, ExecutionError> {
        (self.ptr)(ctx, args)
    }
}

impl<'types> AnnotatedFunction<'types> for NativeFunction<'types> {
    fn name(&self) -> &'static str {
        self.name
    }

    fn location(&self) -> (&'static str, &'static str, &'static str, u32, u32) {
        (
            "melbi-core",
            env!("CARGO_PKG_VERSION"),
            file!(),
            line!(),
            column!(),
        )
    }

    fn doc(&self) -> Option<&str> {
        None
    }
}

pub fn build_array_package<'a, B>(
    arena: &'a Bump,
    type_mgr: &'a TypeManager<'a>,
    mut builder: B,
) -> B
where
    B: Binder<'a, 'a>,
{
    let t = type_mgr.fresh_type_var();
    builder = NativeFunction {
        name: "Len",
        ty: type_mgr.function(&[type_mgr.array(t)], type_mgr.int()),
        ptr: array_len,
    }
    .register(arena, builder);

    let t = type_mgr.fresh_type_var();
    builder = NativeFunction {
        name: "IsEmpty",
        ty: type_mgr.function(&[type_mgr.array(t)], type_mgr.bool()),
        ptr: array_is_empty,
    }
    .register(arena, builder);

    let t = type_mgr.fresh_type_var();
    builder = NativeFunction {
        name: "Slice",
        ty: type_mgr.function(
            &[type_mgr.array(t), type_mgr.int(), type_mgr.int()],
            type_mgr.array(t),
        ),
        ptr: array_slice,
    }
    .register(arena, builder);

    let t = type_mgr.fresh_type_var();
    builder = NativeFunction {
        name: "Concat",
        ty: type_mgr.function(&[type_mgr.array(t), type_mgr.array(t)], type_mgr.array(t)),
        ptr: array_concat,
    }
    .register(arena, builder);

    let t = type_mgr.fresh_type_var();
    builder = NativeFunction {
        name: "Flatten",
        ty: type_mgr.function(&[type_mgr.array(type_mgr.array(t))], type_mgr.array(t)),
        ptr: array_flatten,
    }
    .register(arena, builder);

    let a = type_mgr.fresh_type_var();
    let b = type_mgr.fresh_type_var();
    let tuple_ty = type_mgr.record(vec![("first", a), ("second", b)]);
    builder = NativeFunction {
        name: "Zip",
        ty: type_mgr.function(
            &[type_mgr.array(a), type_mgr.array(b)],
            type_mgr.array(tuple_ty),
        ),
        ptr: array_zip,
    }
    .register(arena, builder);

    let t = type_mgr.fresh_type_var();
    builder = NativeFunction {
        name: "Reverse",
        ty: type_mgr.function(&[type_mgr.array(t)], type_mgr.array(t)),
        ptr: array_reverse,
    }
    .register(arena, builder);

    let t = type_mgr.fresh_type_var();
    let u = type_mgr.fresh_type_var();
    let fn_ty = type_mgr.function(&[t], u);
    builder = NativeFunction {
        name: "Map",
        ty: type_mgr.function(&[type_mgr.array(t), fn_ty], type_mgr.array(u)),
        ptr: array_map,
    }
    .register(arena, builder);

    builder
}

#[cfg(test)]
#[path = "array_test.rs"]
mod array_test;
