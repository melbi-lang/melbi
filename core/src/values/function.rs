//! Function value representation for FFI and future closures.
//!
//! This module defines the `Function` trait which represents callable values in Melbi.
//! Supports native Rust functions, and will support closures, foreign language functions, etc.

use super::dynamic::Value;
use crate::evaluator::ExecutionError;
use crate::types::{Type, manager::TypeManager};
use crate::values::binder::Binder;
use bumpalo::Bump;

// ============================================================================
// FFI Context
// ============================================================================

/// FFI execution context providing access to runtime resources.
///
/// This struct bundles all resources that native functions might need.
/// Functions can request specific resources by including them in their signature,
/// and the `#[melbi_fn]` macro will handle passing only what's needed.
///
/// # Future Extensions
///
/// This struct is designed to be extended with additional resources:
/// - Execution limits (instruction count, memory budget)
/// - Profiling/tracing hooks
/// - External resource handles
///
/// # Example
///
/// ```ignore
/// // Function using full context
/// #[melbi_fn(name = ComplexOp)]
/// fn complex_op(ctx: &FfiContext, data: Array<i64>) -> Value {
///     let result = process(data);
///     Value::array(ctx.arena(), ctx.type_mgr().array(ctx.type_mgr().int()), &result)
/// }
/// ```
pub struct FfiContext<'types, 'arena> {
    arena: &'arena Bump,
    type_mgr: &'types TypeManager<'types>,
}

impl<'types, 'arena> FfiContext<'types, 'arena> {
    /// Create a new FFI context with the given arena and type manager.
    #[inline]
    pub fn new(arena: &'arena Bump, type_mgr: &'types TypeManager<'types>) -> Self {
        Self { arena, type_mgr }
    }

    /// Get the arena for allocating values.
    #[inline]
    pub fn arena(&self) -> &'arena Bump {
        self.arena
    }

    /// Get the type manager for constructing typed values.
    #[inline]
    pub fn type_mgr(&self) -> &'types TypeManager<'types> {
        self.type_mgr
    }
}

// ============================================================================
// Function Trait
// ============================================================================

/// Trait for callable functions in Melbi.
///
/// All callable values (native FFI functions, closures, bytecode lambdas, etc.)
/// implement this trait.
///
/// The `call_unchecked` method uses generic lifetimes to allow functions to work with
/// different arena and type manager lifetimes.
///
/// # Safety
///
/// This trait is intended for internal use by the evaluator. User code should not
/// call `call_unchecked` directly - use the evaluator's function call mechanism instead,
/// which performs proper type checking and argument validation.
pub trait Function<'types, 'arena> {
    /// Returns the function's type signature.
    ///
    /// This type is owned by the implementor and used for runtime validation
    /// in the safe `call()` wrapper (future feature).
    fn ty(&self) -> &'types Type<'types>;

    /// Call the function with the given arguments, without runtime type checking.
    ///
    /// # Safety
    ///
    /// The caller must ensure:
    /// - The number of arguments matches the function's arity
    /// - Each argument's type matches the function's parameter types
    /// - The function type was validated by the type checker
    ///
    /// The evaluator guarantees these invariants, so this is safe when called
    /// from within the evaluator. Direct calls from user code may violate these
    /// invariants and cause panics or incorrect behavior.
    ///
    /// # Parameters
    /// - `ctx`: FFI context providing access to arena and type manager
    /// - `args`: Slice of evaluated arguments (types guaranteed by type checker)
    ///
    /// # Returns
    /// Result containing the return value, or an error that can be caught with `otherwise`.
    #[allow(unsafe_code)]
    unsafe fn call_unchecked(
        &self,
        ctx: &FfiContext<'types, 'arena>,
        args: &[Value<'types, 'arena>],
    ) -> Result<Value<'types, 'arena>, ExecutionError>;
}

/// Type alias for native FFI function pointers.
///
/// This is the signature expected for Rust functions that will be called from Melbi.
///
/// # Example
///
/// ```ignore
/// fn array_len<'types, 'arena>(
///     ctx: &FfiContext<'types, 'arena>,
///     args: &[Value<'types, 'arena>],
/// ) -> Result<Value<'types, 'arena>, EvalError> {
///     assert_eq!(args.len(), 1);
///     assert!(args[0].is_array());
///
///     let array = args[0].as_array().unwrap();
///     Ok(Value::int(ctx.type_mgr(), array.len() as i64))
/// }
/// ```
pub type NativeFn = for<'types, 'arena> fn(
    ctx: &FfiContext<'types, 'arena>,
    args: &[Value<'types, 'arena>],
) -> Result<Value<'types, 'arena>, ExecutionError>;

/// Wrapper for native Rust function pointers.
///
/// Implements the `Function` trait by delegating to the wrapped function pointer.
/// This allows regular Rust functions to be used as Melbi functions.
///
/// # Example
///
/// ```ignore
/// let add_ty = type_mgr.function(&[type_mgr.int(), type_mgr.int()], type_mgr.int());
/// let func = NativeFunction::new(add_ty, array_add);
/// let value = Value::function(&arena, func)?;
/// ```
pub struct NativeFunction<'ty> {
    ty: &'ty Type<'ty>,
    func: NativeFn,
}

impl<'ty> NativeFunction<'ty> {
    /// Create a new native function with its type signature.
    pub fn new(ty: &'ty Type<'ty>, func: NativeFn) -> Self {
        Self { ty, func }
    }
}

impl<'types, 'arena> Function<'types, 'arena> for NativeFunction<'types> {
    fn ty(&self) -> &'types Type<'types> {
        self.ty
    }

    #[allow(unsafe_code)]
    unsafe fn call_unchecked(
        &self,
        ctx: &FfiContext<'types, 'arena>,
        args: &[Value<'types, 'arena>],
    ) -> Result<Value<'types, 'arena>, ExecutionError> {
        // Delegate to the wrapped function pointer
        (self.func)(ctx, args)
    }
}

/// Trait for functions with metadata (name, documentation, source location).
///
/// This trait extends `Function` with metadata useful for:
/// - Function registration in the environment
/// - LSP support (hover, go-to-definition)
/// - Error messages and debugging
///
/// Generated by the `#[melbi_fn]` macro for FFI functions.
pub trait AnnotatedFunction<'types>: Function<'types, 'types> {
    /// The Melbi function name (e.g., "Len", "Sin", "Join").
    fn name(&self) -> &'static str;

    /// Source location where the function was defined (crate, version, file, line, column).
    fn location(&self) -> (&'static str, &'static str, &'static str, u32, u32);

    /// Documentation string extracted from Rust doc comments.
    fn doc(&self) -> Option<&str>;

    /// Register this function as a field in a record builder.
    ///
    /// This default implementation creates a Value from the function and
    /// adds it as a field using the function's name.
    fn register<B>(self, arena: &'types Bump, builder: B) -> B
    where
        Self: Sized + 'types,
        B: Binder<'types, 'types>,
    {
        let name = self.name();
        let value = Value::function(arena, self).expect("`self.ty()` should be a function");
        builder.bind(name, value)
    }
}
