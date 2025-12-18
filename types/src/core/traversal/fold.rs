//! Generic fold (catamorphism) for type traversal and transformation.
//!
//! The `Fold` trait provides a stack-based visitor pattern that can:
//! - Transform types (`Output = Ty<B>`)
//! - Collect information (`Output = HashSet<u16>`)
//! - Perform side effects (`Output = ()`)

use alloc::{vec, vec::Vec};

use crate::{Ty, core::TyBuilder};

/// Control flow for the fold traversal.
///
/// - `Recurse`: Process children, then call `combine`
/// - `Done(out)`: Skip children, push `out` to results stack
/// - `Replace(ty)`: Visit `ty` instead (push to task stack)
pub enum FoldStep<B: TyBuilder, Output> {
    /// Continue into children, then combine results.
    Recurse,
    /// Finished with this node, prune children (push to results stack).
    Done(Output),
    /// Replace with this node for another and continue traversal (push to task stack).
    Replace(Ty<B>),
}

/// A fold (catamorphism) over types.
///
/// The `Output` type determines what kind of fold this is:
/// - `Ty<B>` for type transformations
/// - `()` for side-effect-only traversals
/// - Any other type for computing values
pub trait Fold<B: TyBuilder> {
    type Output;
    type Error;

    /// Called before processing a type's children.
    ///
    /// Return:
    /// - `FoldStep::Recurse` to process children and call `combine`
    /// - `FoldStep::Done(out)` to skip children and use `out` as result
    /// - `FoldStep::Replace(ty)` to visit `ty` instead
    fn visit(&mut self, builder: &B, ty: &Ty<B>) -> Result<FoldStep<B, Self::Output>, Self::Error>;

    /// Called after all children have been processed.
    ///
    /// `children` contains results from child types in definition order:
    /// - `Array`: `[element]`
    /// - `Map`: `[key, value]`
    /// - `Record`: `[field0, field1, ...]`
    /// - `Function`: `[param0, param1, ..., ret]`
    /// - Leaves (TypeVar, Scalar, Symbol): `[]`
    fn combine(
        &mut self,
        builder: &B,
        ty: &Ty<B>,
        children: impl ExactSizeIterator<Item = Self::Output> + DoubleEndedIterator,
    ) -> Result<Self::Output, Self::Error>;
}

enum Task<B: TyBuilder> {
    Visit(Ty<B>),
    Combine(usize, Ty<B>),
}

/// Drive a fold over a type tree using stack-based iteration.
///
/// This avoids stack overflow for deeply nested types.
pub fn drive_fold<B, F>(builder: &B, root: Ty<B>, mut folder: F) -> Result<F::Output, F::Error>
where
    B: TyBuilder,
    F: Fold<B>,
{
    let mut stack = vec![Task::<B>::Visit(root)];
    let mut results: Vec<F::Output> = Vec::new();

    while let Some(task) = stack.pop() {
        match task {
            Task::Visit(ty) => match folder.visit(builder, &ty)? {
                FoldStep::Done(out) => {
                    results.push(out);
                }
                FoldStep::Replace(new_ty) => {
                    stack.push(Task::Visit(new_ty));
                }
                FoldStep::Recurse => {
                    let children = B::resolve_ty_node(&ty).kind().iter_children();
                    stack.push(Task::Combine(children.len(), ty.clone()));
                    stack.extend(children.rev().map(move |child| Task::Visit(child.clone())));
                }
            },
            Task::Combine(count, ty) => {
                let start = results
                    .len()
                    .checked_sub(count)
                    .expect("Bug: result stack underflow");
                let children = results.drain(start..);
                let out = folder.combine(builder, &ty, children)?;
                results.push(out);
            }
        }
    }

    debug_assert_eq!(
        results.len(),
        1,
        "Algorithm bug: expected exactly one result"
    );
    Ok(results.pop().expect("empty result stack"))
}

/// Simplified fold for `Ty<B> -> Ty<B>` transformations.
pub trait TypeFolder<In: TyBuilder, Out: TyBuilder = In> {
    /// Return `Some(ty)` to replace and continue into `ty`, or `None` to recurse.
    fn fold_ty(&mut self, builder_in: &In, builder_out: &Out, ty: &Ty<In>)
    -> FoldStep<In, Ty<Out>>;
}

struct TypeFolderAdapter<'a, In: TyBuilder, Out: TyBuilder, F: TypeFolder<In, Out>> {
    folder: &'a mut F,
    builder_out: &'a Out,
    _marker: core::marker::PhantomData<(In, Out)>,
}

impl<'a, In, Out, F> Fold<In> for TypeFolderAdapter<'a, In, Out, F>
where
    In: TyBuilder,
    Out: TyBuilder,
    F: TypeFolder<In, Out>,
{
    type Output = Ty<Out>;
    type Error = ();

    fn visit(&mut self, builder: &In, ty: &Ty<In>) -> Result<FoldStep<In, Ty<Out>>, ()> {
        Ok(self.folder.fold_ty(builder, self.builder_out, ty))
    }

    fn combine(
        &mut self,
        _builder: &In,
        ty: &Ty<In>,
        children: impl ExactSizeIterator<Item = Ty<Out>> + DoubleEndedIterator,
    ) -> Result<Ty<Out>, ()> {
        let new_ty = ty
            .kind()
            .from_iter_children::<Out>(self.builder_out, children)
            .alloc(self.builder_out);
        Ok(new_ty)
    }
}

/// Fold a type using a `TypeFolder`.
pub fn fold_type<F, In, Out>(
    builder_in: &In,
    builder_out: &Out,
    root: Ty<In>,
    folder: &mut F,
) -> Ty<Out>
where
    In: TyBuilder,
    Out: TyBuilder,
    F: TypeFolder<In, Out>,
{
    let adapter = TypeFolderAdapter {
        folder,
        builder_out,
        _marker: core::marker::PhantomData,
    };
    drive_fold(builder_in, root, adapter).expect("shouldn't fail")
}
