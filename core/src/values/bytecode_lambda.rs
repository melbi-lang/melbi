//! Bytecode lambda function implementation for closures.
//!
//! This module defines `BytecodeLambda` which represents Melbi lambdas compiled to bytecode.
//! When called, it creates a new VM to execute the lambda's Code with captures.
//!
//! # Polymorphism Support
//!
//! Polymorphic lambdas store multiple compiled instantiations, one per unique type
//! instantiation observed at call sites. At runtime, `call_unchecked` selects the
//! appropriate instantiation based on argument types.

use bumpalo::Bump;

use super::dynamic::Value;
use super::function::{FfiContext, Function};
use crate::evaluator::ExecutionError;
use crate::types::Type;
use crate::types::traits::{TypeKind, TypeView};
use crate::values::RawValue;
use crate::vm::{Code, VM};

/// A single compiled instantiation of a lambda.
///
/// For polymorphic lambdas, there's one entry per unique type instantiation.
/// For monomorphic lambdas, there's exactly one entry.
#[derive(Clone, Copy)]
pub struct LambdaInstantiation<'types, 'arena> {
    /// The concrete function type for this instantiation
    pub fn_type: &'types Type<'types>,
    /// The compiled Code for this instantiation
    pub code: &'arena Code<'types>,
}

/// A bytecode-compiled lambda function value.
///
/// Stores the lambda's type signature, compiled Code instantiations, and captured values.
/// When called, it creates a new VM with captures and executes the appropriate Code.
///
/// # Closure Support
///
/// Lambdas can capture variables from their enclosing scope. Captured variables are stored
/// as a slice of `RawValues` and passed to the VM when the lambda is called.
///
/// # Polymorphism
///
/// For polymorphic lambdas (e.g., `(x) => x`), multiple Code instantiations are stored,
/// one per unique type instantiation. At call time, the appropriate instantiation is
/// selected based on argument types.
pub struct BytecodeLambda<'types, 'arena> {
    /// The function's generic type signature (may contain type variables for polymorphic lambdas)
    ty: &'types Type<'types>,

    /// All compiled instantiations of this lambda.
    /// For monomorphic lambdas: single entry.
    /// For polymorphic lambdas: one entry per unique type instantiation.
    instantiations: &'arena [LambdaInstantiation<'types, 'arena>],

    /// Captured values from the enclosing scope
    captures: &'arena [RawValue],
}

impl<'types, 'arena> BytecodeLambda<'types, 'arena> {
    /// Create a new bytecode lambda with multiple instantiations.
    ///
    /// # Parameters
    ///
    /// - `ty`: The function's type (must be a Function type)
    /// - `instantiations`: All compiled instantiations (concrete type + Code)
    /// - `captures`: Captured values from the enclosing scope
    #[must_use]
    pub fn new(
        ty: &'types Type<'types>,
        instantiations: &'arena [LambdaInstantiation<'types, 'arena>],
        captures: &'arena [RawValue],
    ) -> Self {
        debug_assert!(
            matches!(ty, Type::Function { .. }),
            "BytecodeLambda type must be Function"
        );
        debug_assert!(
            !instantiations.is_empty(),
            "BytecodeLambda must have at least one instantiation"
        );

        Self {
            ty,
            instantiations,
            captures,
        }
    }

    /// Create a monomorphic lambda with a single instantiation.
    ///
    /// Convenience constructor for the common case of non-polymorphic lambdas.
    pub fn new_monomorphic(
        arena: &'arena Bump,
        ty: &'types Type<'types>,
        code: &'arena Code<'types>,
        captures: &'arena [RawValue],
    ) -> Self {
        let instantiation = LambdaInstantiation { fn_type: ty, code };
        let instantiations = arena.alloc_slice_copy(&[instantiation]);
        Self::new(ty, instantiations, captures)
    }

    /// Find the instantiation matching the given argument types.
    ///
    /// For monomorphic lambdas, this always returns the single instantiation.
    /// For polymorphic lambdas, this finds the instantiation whose parameter types
    /// match the runtime argument types.
    fn find_instantiation(
        &self,
        args: &[Value<'types, 'arena>],
    ) -> &LambdaInstantiation<'types, 'arena> {
        if self.instantiations.len() == 1 {
            // Fast path for monomorphic lambdas
            return &self.instantiations[0];
        }

        let arg_types: alloc::vec::Vec<_> = args.iter().map(|a| a.ty).collect();
        tracing::trace!(args = ?arg_types, "find_instantiation: looking for args");

        // For polymorphic lambdas, find matching instantiation by argument types
        for (i, inst) in self.instantiations.iter().enumerate() {
            if let TypeKind::Function { params, .. } = inst.fn_type.view() {
                let params: alloc::vec::Vec<_> = params.collect();
                tracing::trace!(inst = i, params = ?params, fn_type = %inst.fn_type, "find_instantiation: checking");
                if params.len() == args.len()
                    && params.iter().zip(args.iter()).all(|(p, a)| *p == a.ty)
                {
                    tracing::trace!(inst = i, "find_instantiation: MATCH");
                    return inst;
                }
            }
        }

        // Type checker guarantees a match exists
        panic!(
            "No matching instantiation for argument types: {:?}",
            args.iter().map(|a| a.ty).collect::<alloc::vec::Vec<_>>()
        );
    }
}

impl<'types, 'arena> Function<'types, 'arena> for BytecodeLambda<'types, 'arena> {
    fn ty(&self) -> &'types Type<'types> {
        self.ty
    }

    unsafe fn call_unchecked(
        &self,
        ctx: &FfiContext<'types, 'arena>,
        args: &[Value<'types, 'arena>],
    ) -> Result<Value<'types, 'arena>, ExecutionError> {
        // Find the appropriate instantiation for these argument types
        let inst = self.find_instantiation(args);

        tracing::trace!(fn_type = %inst.fn_type, code = ?inst.code, "call_unchecked: selected instantiation");

        // Collect arguments as locals
        let locals = args.iter().map(super::dynamic::Value::as_raw).collect();

        // Create VM with locals and captures, then execute
        let mut vm = VM::new(ctx.arena(), inst.code, locals, self.captures);
        let result = vm.run()?;

        tracing::trace!(result = ?result, "call_unchecked: result raw");

        // Convert RawValue back to Value using the instantiation's return type
        let return_type = match inst.fn_type.view() {
            TypeKind::Function { ret, .. } => ret,
            _ => unreachable!("BytecodeLambda type must be Function"),
        };

        tracing::trace!(return_type = %return_type, "call_unchecked: return type");

        // SAFETY: The type is derived from the instantiation's return type, which is correct
        // for the result of evaluating the lambda body with these argument types.
        Ok(Value::from_raw_unchecked(return_type, result))
    }
}
