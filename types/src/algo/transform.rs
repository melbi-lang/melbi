use alloc::{vec, vec::Vec};

use crate::{
    kind::TyKind,
    traits::{Ty, TyBuilder},
};

pub enum VisitStep<B: TyBuilder> {
    Return(Ty<B>),
    Replace(Ty<B>),
    Recurse,
}

pub trait Visitor<B: TyBuilder> {
    type Output;

    fn visit(&mut self, builder: &B, ty: &Ty<B>) -> VisitStep<B>;
    fn combine(&mut self, builder: &B, ty: &Ty<B>, result: Self::Output) -> Self::Output;
}

enum Task<'a, B: TyBuilder> {
    Visit(&'a Ty<B>),
    Combine(TyKind<B>), // Should be a "template" actually
}

pub fn drive_visitor<B, V>(builder: &B, root: &Ty<B>, mut visitor: V) -> V::Output
where
    B: TyBuilder,
    V: Visitor<B>,
{
    let mut stack = vec![Task::Visit(root)];
    let mut results = Vec::new();

    while let Some(task) = stack.pop() {
        match task {
            Task::Visit(ty) => {
                match visitor.visit(builder, ty) {
                    // A. Prune/Leaf: Push result, don't recurse
                    VisitStep::Return(val) => results.push(val),

                    // B. Replace: Push the NEW handle to be visited
                    VisitStep::Replace(new_handle) => stack.push(Task::Visit(&new_handle)),

                    // C. Standard Recursion
                    VisitStep::Recurse => {
                        let ty_kind = ty.node().kind();

                        // Push "PostVisit" (so it runs AFTER children)
                        stack.push(Task::Combine(ty_kind.clone()));

                        // Push children (in reverse, so they pop in order)
                        kind.push_children_rev(&mut stack);
                    }
                }
            }
            Task::PostVisit(kind) => {
                // 2. Gather Results from Children
                let count = kind.child_count();
                // Drain the top 'count' results
                let start = results.len() - count;
                let child_results: Vec<_> = results.drain(start..).collect();

                // 3. User Hook: Exit (Combine/Rebuild)
                let final_res = visitor.exit(builder, kind, child_results);
                results.push(final_res);
            }
        }
    }

    results
        .pop()
        .expect("Algorithm failure: Stack empty but no result")
}
