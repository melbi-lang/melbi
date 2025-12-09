use alloc::{vec, vec::Vec};

use crate::{
    kind::TyKind,
    traits::{Ty, TyBuilder},
};

pub trait Visitor<B: TyBuilder> {
    type Output;
}

enum Task<'a, 'b, B: TyBuilder> {
    Visit(&'a Ty<B>),
    Combine(&'b Ty<B>),
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
                match visitor.enter(builder, ty) {
                    // A. Prune/Leaf: Push result, don't recurse
                    VisitStep::Return(val) => results.push(val),

                    // B. Replace: Push the NEW handle to be visited
                    VisitStep::Replace(new_handle) => stack.push(Task::Visit(new_handle)),

                    // C. Standard Recursion
                    VisitStep::Recurse => {
                        let (_, kind) = builder.resolve(&handle);

                        // Push "PostVisit" (so it runs AFTER children)
                        stack.push(Task::PostVisit(kind.clone()));

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
