//! Folding the parsed AST: rebuild it, retarget it at another builder, or
//! rewrite parts of it in place.
//!
//! Adapted from `types/src/core/traversal/fold.rs`. Two things had to change for
//! trees that are not homogeneous.
//!
//! **The result stack is split per descriptor.** A type's children are all
//! types, so one `Vec<Output>` sufficed there. An expression's children span
//! several trees, so there are seven stacks, each holding exactly one
//! descriptor's output. Nothing is type-erased on the way out: `combine` pops
//! `Tree<Out, Expr>` from the expression stack and `Tree<Out, MatchArm>` from the
//! arm stack, already typed.
//!
//! **Only the task stack is an enum.** That enum is private to the driver and
//! never reaches a folder, which sees `fold_expr(&Tree<In, Expr>)` and friends
//! with fully typed signatures.
//!
//! The traversal is iterative, so nesting depth costs heap rather than native
//! stack — the property `drive_fold` was written for, preserved here.
//!
//! There is deliberately no `iter_children`. Materialising children into an
//! iterator is more code than matching on the kind, and for heterogeneous
//! children it cannot even be one iterator type.

use alloc::vec::Vec;

use crate::{Tree, TreeBuilder, TreeNode};

use super::{
    Binding, BindingKind, Data, Expr, ExprKind, LiteralKind, MapEntry, MapEntryKind, MatchArm,
    MatchArmKind, Pattern, PatternKind, TypeExpr, TypeExprKind, TypeField, TypeFieldKind,
};

/// What a `fold_*` method decides about one node.
///
/// Mirrors `FoldStep` in `melbi-types`, with the output pinned to a tree of the
/// same descriptor in the output builder.
pub enum Step<In: TreeBuilder, Out: TreeBuilder, D: crate::TreeDescriptor> {
    /// Fold the children, then rebuild this node around the results.
    Recurse,
    /// Finished — use this as the result and do not look at the children.
    Done(Tree<Out, D>),
    /// Fold this node instead, from the top. Used for chained substitution.
    Replace(Tree<In, D>),
}

/// A rebuilding traversal over the parsed AST.
///
/// Every method defaults to [`Step::Recurse`], so a pass overrides only the
/// trees it cares about and the driver handles the rest. This is what makes the
/// three shapes of pass all the same trait:
///
/// - **Retarget at another builder** (`In ≠ Out`): override nothing. The driver
///   rebuilds every node in `Out`. This is the arena-to-heap copy.
/// - **Rewrite in place** (`In = Out`): override the cases you rewrite and
///   return [`Step::Done`] or [`Step::Replace`]; everything else is rebuilt
///   unchanged in the same builder.
/// - **Rewrite one tree only**: override `fold_pattern`, leave the other six.
///
/// A pass that computes a value rather than a tree wants `Visitor` instead —
/// this trait always produces a tree.
pub trait Folder<In: TreeBuilder, Out: TreeBuilder = In> {
    type Error;

    /// The builder every rebuilt node is allocated into.
    fn output_builder(&self) -> &Out;

    /// Map a node's data across the stage boundary.
    ///
    /// Both stages use [`Data`] today, so this defaults to copying. A lowering
    /// pass that changes the data shape overrides it.
    fn fold_data(&mut self, data: &Data) -> Result<Data, Self::Error> {
        Ok(*data)
    }

    fn fold_expr(&mut self, tree: &Tree<In, Expr>) -> Result<Step<In, Out, Expr>, Self::Error> {
        let _ = tree;
        Ok(Step::Recurse)
    }

    fn fold_pattern(
        &mut self,
        tree: &Tree<In, Pattern>,
    ) -> Result<Step<In, Out, Pattern>, Self::Error> {
        let _ = tree;
        Ok(Step::Recurse)
    }

    fn fold_match_arm(
        &mut self,
        tree: &Tree<In, MatchArm>,
    ) -> Result<Step<In, Out, MatchArm>, Self::Error> {
        let _ = tree;
        Ok(Step::Recurse)
    }

    fn fold_binding(
        &mut self,
        tree: &Tree<In, Binding>,
    ) -> Result<Step<In, Out, Binding>, Self::Error> {
        let _ = tree;
        Ok(Step::Recurse)
    }

    fn fold_map_entry(
        &mut self,
        tree: &Tree<In, MapEntry>,
    ) -> Result<Step<In, Out, MapEntry>, Self::Error> {
        let _ = tree;
        Ok(Step::Recurse)
    }

    fn fold_type_expr(
        &mut self,
        tree: &Tree<In, TypeExpr>,
    ) -> Result<Step<In, Out, TypeExpr>, Self::Error> {
        let _ = tree;
        Ok(Step::Recurse)
    }

    fn fold_type_field(
        &mut self,
        tree: &Tree<In, TypeField>,
    ) -> Result<Step<In, Out, TypeField>, Self::Error> {
        let _ = tree;
        Ok(Step::Recurse)
    }
}

// =============================================================================
// The driver
// =============================================================================

/// One unit of work. Private: a folder never sees this, which is what keeps the
/// erasure out of pass code.
enum Task<In: TreeBuilder> {
    Expr(Tree<In, Expr>),
    Pattern(Tree<In, Pattern>),
    MatchArm(Tree<In, MatchArm>),
    Binding(Tree<In, Binding>),
    MapEntry(Tree<In, MapEntry>),
    TypeExpr(Tree<In, TypeExpr>),
    TypeField(Tree<In, TypeField>),

    /// Children are done and sit at the end of their stacks; rebuild this node.
    Combine(Combine<In>),
}

enum Combine<In: TreeBuilder> {
    Expr(Tree<In, Expr>),
    Pattern(Tree<In, Pattern>),
    MatchArm(Tree<In, MatchArm>),
    Binding(Tree<In, Binding>),
    MapEntry(Tree<In, MapEntry>),
    TypeExpr(Tree<In, TypeExpr>),
    TypeField(Tree<In, TypeField>),
}

/// Seven typed stacks. Children of one node land contiguously at the end of the
/// stack for their descriptor, in source order, so `combine` drains a known
/// count from the back.
struct Results<Out: TreeBuilder> {
    exprs: Vec<Tree<Out, Expr>>,
    patterns: Vec<Tree<Out, Pattern>>,
    arms: Vec<Tree<Out, MatchArm>>,
    bindings: Vec<Tree<Out, Binding>>,
    entries: Vec<Tree<Out, MapEntry>>,
    type_exprs: Vec<Tree<Out, TypeExpr>>,
    type_fields: Vec<Tree<Out, TypeField>>,
}

impl<Out: TreeBuilder> Results<Out> {
    fn new() -> Self {
        Self {
            exprs: Vec::new(),
            patterns: Vec::new(),
            arms: Vec::new(),
            bindings: Vec::new(),
            entries: Vec::new(),
            type_exprs: Vec::new(),
            type_fields: Vec::new(),
        }
    }
}

/// Drains the last `n` items of a stack, in the order they were pushed.
macro_rules! take {
    ($stack:expr, $n:expr) => {{
        let n = $n;
        let start = $stack
            .len()
            .checked_sub(n)
            .expect("driver bug: result stack underflow");
        $stack.drain(start..)
    }};
}

/// Pops exactly one result, the common single-child case.
macro_rules! take_one {
    ($stack:expr) => {
        $stack.pop().expect("driver bug: result stack underflow")
    };
}

macro_rules! drive_entry_point {
    ($name:ident, $descriptor:ty, $task:ident, $stack:ident) => {
        /// Fold a tree, returning the rebuilt root.
        pub fn $name<In, Out, F>(
            root: &Tree<In, $descriptor>,
            folder: &mut F,
        ) -> Result<Tree<Out, $descriptor>, F::Error>
        where
            In: TreeBuilder,
            Out: TreeBuilder,
            F: Folder<In, Out>,
        {
            let mut stack = alloc::vec![Task::$task(root.clone())];
            let mut results = Results::new();
            run(&mut stack, &mut results, folder)?;
            Ok(take_one!(results.$stack))
        }
    };
}

drive_entry_point!(fold_expr, Expr, Expr, exprs);
drive_entry_point!(fold_pattern, Pattern, Pattern, patterns);
drive_entry_point!(fold_match_arm, MatchArm, MatchArm, arms);
drive_entry_point!(fold_binding, Binding, Binding, bindings);
drive_entry_point!(fold_map_entry, MapEntry, MapEntry, entries);
drive_entry_point!(fold_type_expr, TypeExpr, TypeExpr, type_exprs);
drive_entry_point!(fold_type_field, TypeField, TypeField, type_fields);

/// Dispatches one `fold_*` call and pushes the follow-up work.
///
/// The three arms are the same for every descriptor: `Done` pushes the result
/// straight onto the stack, `Replace` re-queues the substitute, and `Recurse`
/// queues a combine followed by the children — pushed last so they pop first.
macro_rules! step {
    ($folder:ident, $hook:ident, $tree:expr, $stack:expr, $results:expr,
     $out_stack:ident, $combine:ident, $push_children:expr) => {{
        let tree = $tree;
        match $folder.$hook(&tree)? {
            Step::Done(out) => $results.$out_stack.push(out),
            Step::Replace(other) => $stack.push(Task::$combine(other)),
            Step::Recurse => {
                $stack.push(Task::Combine(Combine::$combine(tree.clone())));
                #[allow(clippy::redundant_closure_call)]
                ($push_children)(&tree, $stack);
            }
        }
    }};
}

fn run<In, Out, F>(
    stack: &mut Vec<Task<In>>,
    results: &mut Results<Out>,
    folder: &mut F,
) -> Result<(), F::Error>
where
    In: TreeBuilder,
    Out: TreeBuilder,
    F: Folder<In, Out>,
{
    while let Some(task) = stack.pop() {
        match task {
            Task::Expr(tree) => step!(
                folder,
                fold_expr,
                tree,
                stack,
                results,
                exprs,
                Expr,
                push_expr_children
            ),
            Task::Pattern(tree) => step!(
                folder,
                fold_pattern,
                tree,
                stack,
                results,
                patterns,
                Pattern,
                push_pattern_children
            ),
            Task::MatchArm(tree) => step!(
                folder,
                fold_match_arm,
                tree,
                stack,
                results,
                arms,
                MatchArm,
                push_match_arm_children
            ),
            Task::Binding(tree) => step!(
                folder,
                fold_binding,
                tree,
                stack,
                results,
                bindings,
                Binding,
                push_binding_children
            ),
            Task::MapEntry(tree) => step!(
                folder,
                fold_map_entry,
                tree,
                stack,
                results,
                entries,
                MapEntry,
                push_map_entry_children
            ),
            Task::TypeExpr(tree) => step!(
                folder,
                fold_type_expr,
                tree,
                stack,
                results,
                type_exprs,
                TypeExpr,
                push_type_expr_children
            ),
            Task::TypeField(tree) => step!(
                folder,
                fold_type_field,
                tree,
                stack,
                results,
                type_fields,
                TypeField,
                push_type_field_children
            ),

            Task::Combine(what) => combine(what, results, folder)?,
        }
    }
    Ok(())
}

// --- Walking: push a node's children, last-first so they pop in order --------
//
// Written as a match per kind rather than an iterator, deliberately: the match
// *is* the walk skeleton, and it is less code than materialising an iterator
// that could not be one type anyway.

fn push_expr_children<In: TreeBuilder>(tree: &Tree<In, Expr>, stack: &mut Vec<Task<In>>) {
    match tree.kind() {
        ExprKind::Ident(_) | ExprKind::None => {}
        ExprKind::Literal(literal) => push_literal_children(literal, stack),

        ExprKind::Unary { expr, .. } | ExprKind::Some(expr) => stack.push(Task::Expr(expr.clone())),
        ExprKind::Field { value, .. } => stack.push(Task::Expr(value.clone())),
        ExprKind::Lambda { body, .. } => stack.push(Task::Expr(body.clone())),

        ExprKind::Binary { left, right, .. }
        | ExprKind::Boolean { left, right, .. }
        | ExprKind::Comparison { left, right, .. } => {
            stack.push(Task::Expr(right.clone()));
            stack.push(Task::Expr(left.clone()));
        }
        ExprKind::Index { value, index } => {
            stack.push(Task::Expr(index.clone()));
            stack.push(Task::Expr(value.clone()));
        }
        ExprKind::Otherwise { primary, fallback } => {
            stack.push(Task::Expr(fallback.clone()));
            stack.push(Task::Expr(primary.clone()));
        }
        ExprKind::Cast { expr, ty } => {
            stack.push(Task::TypeExpr(ty.clone()));
            stack.push(Task::Expr(expr.clone()));
        }
        ExprKind::If {
            cond,
            then_branch,
            else_branch,
        } => {
            stack.push(Task::Expr(else_branch.clone()));
            stack.push(Task::Expr(then_branch.clone()));
            stack.push(Task::Expr(cond.clone()));
        }

        ExprKind::Call { callable, args } => {
            for arg in args.iter().rev() {
                stack.push(Task::Expr(arg.clone()));
            }
            stack.push(Task::Expr(callable.clone()));
        }
        ExprKind::Array(items) | ExprKind::FormatStr { exprs: items, .. } => {
            for item in items.iter().rev() {
                stack.push(Task::Expr(item.clone()));
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            for arm in arms.iter().rev() {
                stack.push(Task::MatchArm(arm.clone()));
            }
            stack.push(Task::Expr(scrutinee.clone()));
        }
        ExprKind::Where { expr, bindings } => {
            for binding in bindings.iter().rev() {
                stack.push(Task::Binding(binding.clone()));
            }
            stack.push(Task::Expr(expr.clone()));
        }
        ExprKind::Record(bindings) => {
            for binding in bindings.iter().rev() {
                stack.push(Task::Binding(binding.clone()));
            }
        }
        ExprKind::Map(entries) => {
            for entry in entries.iter().rev() {
                stack.push(Task::MapEntry(entry.clone()));
            }
        }
    }
}

/// A literal is inline, so it has no task of its own — only its unit suffix,
/// which is an expression.
fn push_literal_children<In: TreeBuilder>(literal: &LiteralKind<In>, stack: &mut Vec<Task<In>>) {
    if let Some(suffix) = literal.suffix() {
        stack.push(Task::Expr(suffix.clone()));
    }
}

fn push_pattern_children<In: TreeBuilder>(tree: &Tree<In, Pattern>, stack: &mut Vec<Task<In>>) {
    match tree.kind() {
        PatternKind::Wildcard | PatternKind::Binding(_) | PatternKind::None => {}
        PatternKind::Literal(literal) => push_literal_children(literal, stack),
        PatternKind::Some(inner) => stack.push(Task::Pattern(inner.clone())),
    }
}

fn push_match_arm_children<In: TreeBuilder>(tree: &Tree<In, MatchArm>, stack: &mut Vec<Task<In>>) {
    let arm = tree.kind();
    stack.push(Task::Expr(arm.body.clone()));
    stack.push(Task::Pattern(arm.pattern.clone()));
}

fn push_binding_children<In: TreeBuilder>(tree: &Tree<In, Binding>, stack: &mut Vec<Task<In>>) {
    stack.push(Task::Expr(tree.kind().value.clone()));
}

fn push_map_entry_children<In: TreeBuilder>(tree: &Tree<In, MapEntry>, stack: &mut Vec<Task<In>>) {
    let entry = tree.kind();
    stack.push(Task::Expr(entry.value.clone()));
    stack.push(Task::Expr(entry.key.clone()));
}

fn push_type_expr_children<In: TreeBuilder>(tree: &Tree<In, TypeExpr>, stack: &mut Vec<Task<In>>) {
    match tree.kind() {
        TypeExprKind::Path(_) => {}
        TypeExprKind::Parametrized { params, .. } => {
            for param in params.iter().rev() {
                stack.push(Task::TypeExpr(param.clone()));
            }
        }
        TypeExprKind::Record(fields) => {
            for field in fields.iter().rev() {
                stack.push(Task::TypeField(field.clone()));
            }
        }
    }
}

fn push_type_field_children<In: TreeBuilder>(
    tree: &Tree<In, TypeField>,
    stack: &mut Vec<Task<In>>,
) {
    stack.push(Task::TypeExpr(tree.kind().ty.clone()));
}

// --- Combining: rebuild a node around the results its children left behind ---

fn combine<In, Out, F>(
    what: Combine<In>,
    results: &mut Results<Out>,
    folder: &mut F,
) -> Result<(), F::Error>
where
    In: TreeBuilder,
    Out: TreeBuilder,
    F: Folder<In, Out>,
{
    match what {
        Combine::Expr(tree) => {
            let kind = rebuild_expr(tree.kind(), results, folder)?;
            let data = folder.fold_data(tree.data())?;
            let out = TreeNode::new(data, kind).alloc(folder.output_builder());
            results.exprs.push(out);
        }
        Combine::Pattern(tree) => {
            let kind = rebuild_pattern(tree.kind(), results, folder)?;
            let data = folder.fold_data(tree.data())?;
            let out = TreeNode::new(data, kind).alloc(folder.output_builder());
            results.patterns.push(out);
        }
        Combine::MatchArm(tree) => {
            let body = take_one!(results.exprs);
            let pattern = take_one!(results.patterns);
            let data = folder.fold_data(tree.data())?;
            let out =
                TreeNode::new(data, MatchArmKind { pattern, body }).alloc(folder.output_builder());
            results.arms.push(out);
        }
        Combine::Binding(tree) => {
            let value = take_one!(results.exprs);
            let name = folder.output_builder().alloc_str(tree.kind().name.as_ref());
            let data = folder.fold_data(tree.data())?;
            let out =
                TreeNode::new(data, BindingKind { name, value }).alloc(folder.output_builder());
            results.bindings.push(out);
        }
        Combine::MapEntry(tree) => {
            let value = take_one!(results.exprs);
            let key = take_one!(results.exprs);
            let data = folder.fold_data(tree.data())?;
            let out =
                TreeNode::new(data, MapEntryKind { key, value }).alloc(folder.output_builder());
            results.entries.push(out);
        }
        Combine::TypeExpr(tree) => {
            let kind = rebuild_type_expr(tree.kind(), results, folder)?;
            let data = folder.fold_data(tree.data())?;
            let out = TreeNode::new(data, kind).alloc(folder.output_builder());
            results.type_exprs.push(out);
        }
        Combine::TypeField(tree) => {
            let ty = take_one!(results.type_exprs);
            let name = folder.output_builder().alloc_str(tree.kind().name.as_ref());
            let data = folder.fold_data(tree.data())?;
            let out =
                TreeNode::new(data, TypeFieldKind { name, ty }).alloc(folder.output_builder());
            results.type_fields.push(out);
        }
    }
    Ok(())
}

fn rebuild_literal<In, Out, F>(
    literal: &LiteralKind<In>,
    results: &mut Results<Out>,
    folder: &mut F,
) -> Result<LiteralKind<Out>, F::Error>
where
    In: TreeBuilder,
    Out: TreeBuilder,
    F: Folder<In, Out>,
{
    let out = folder.output_builder();
    Ok(match literal {
        LiteralKind::Int { value, suffix } => LiteralKind::Int {
            value: *value,
            suffix: suffix.as_ref().map(|_| take_one!(results.exprs)),
        },
        LiteralKind::Float { value, suffix } => LiteralKind::Float {
            value: *value,
            suffix: suffix.as_ref().map(|_| take_one!(results.exprs)),
        },
        LiteralKind::Bool(b) => LiteralKind::Bool(*b),
        LiteralKind::Str(s) => LiteralKind::Str(out.alloc_str(s.as_ref())),
        LiteralKind::Bytes(bytes) => LiteralKind::Bytes(out.alloc_bytes(bytes)),
    })
}

fn rebuild_expr<In, Out, F>(
    kind: &ExprKind<In>,
    results: &mut Results<Out>,
    folder: &mut F,
) -> Result<ExprKind<Out>, F::Error>
where
    In: TreeBuilder,
    Out: TreeBuilder,
    F: Folder<In, Out>,
{
    let out = folder.output_builder().clone();
    Ok(match kind {
        ExprKind::Ident(name) => ExprKind::Ident(out.alloc_str(name.as_ref())),
        ExprKind::None => ExprKind::None,
        ExprKind::Literal(literal) => ExprKind::Literal(rebuild_literal(literal, results, folder)?),

        ExprKind::Unary { op, .. } => ExprKind::Unary {
            op: *op,
            expr: take_one!(results.exprs),
        },
        ExprKind::Some(_) => ExprKind::Some(take_one!(results.exprs)),
        ExprKind::Field { field, .. } => ExprKind::Field {
            value: take_one!(results.exprs),
            field: out.alloc_str(field.as_ref()),
        },
        ExprKind::Lambda { params, .. } => ExprKind::Lambda {
            params: out.alloc_str_list(
                params
                    .iter()
                    .map(|p| out.alloc_str(p.as_ref()))
                    .collect::<Vec<_>>(),
            ),
            body: take_one!(results.exprs),
        },

        ExprKind::Binary { op, .. } => {
            let (left, right) = take_two(&mut results.exprs);
            ExprKind::Binary {
                op: *op,
                left,
                right,
            }
        }
        ExprKind::Boolean { op, .. } => {
            let (left, right) = take_two(&mut results.exprs);
            ExprKind::Boolean {
                op: *op,
                left,
                right,
            }
        }
        ExprKind::Comparison { op, .. } => {
            let (left, right) = take_two(&mut results.exprs);
            ExprKind::Comparison {
                op: *op,
                left,
                right,
            }
        }
        ExprKind::Index { .. } => {
            let (value, index) = take_two(&mut results.exprs);
            ExprKind::Index { value, index }
        }
        ExprKind::Otherwise { .. } => {
            let (primary, fallback) = take_two(&mut results.exprs);
            ExprKind::Otherwise { primary, fallback }
        }
        ExprKind::Cast { .. } => ExprKind::Cast {
            ty: take_one!(results.type_exprs),
            expr: take_one!(results.exprs),
        },
        ExprKind::If { .. } => {
            let start = results.exprs.len() - 3;
            let mut it = results.exprs.drain(start..);
            ExprKind::If {
                cond: it.next().unwrap(),
                then_branch: it.next().unwrap(),
                else_branch: it.next().unwrap(),
            }
        }

        ExprKind::Call { args, .. } => {
            let args = out.alloc_list(take!(results.exprs, args.len()).collect::<Vec<_>>());
            ExprKind::Call {
                callable: take_one!(results.exprs),
                args,
            }
        }
        ExprKind::Array(items) => {
            ExprKind::Array(out.alloc_list(take!(results.exprs, items.len()).collect::<Vec<_>>()))
        }
        ExprKind::FormatStr { strs, exprs } => ExprKind::FormatStr {
            exprs: out.alloc_list(take!(results.exprs, exprs.len()).collect::<Vec<_>>()),
            strs: out.alloc_str_list(
                strs.iter()
                    .map(|s| out.alloc_str(s.as_ref()))
                    .collect::<Vec<_>>(),
            ),
        },
        ExprKind::Match { arms, .. } => {
            let arms = out.alloc_list(take!(results.arms, arms.len()).collect::<Vec<_>>());
            ExprKind::Match {
                scrutinee: take_one!(results.exprs),
                arms,
            }
        }
        ExprKind::Where { bindings, .. } => {
            let bindings =
                out.alloc_list(take!(results.bindings, bindings.len()).collect::<Vec<_>>());
            ExprKind::Where {
                expr: take_one!(results.exprs),
                bindings,
            }
        }
        ExprKind::Record(bindings) => ExprKind::Record(
            out.alloc_list(take!(results.bindings, bindings.len()).collect::<Vec<_>>()),
        ),
        ExprKind::Map(entries) => {
            ExprKind::Map(out.alloc_list(take!(results.entries, entries.len()).collect::<Vec<_>>()))
        }
    })
}

fn rebuild_pattern<In, Out, F>(
    kind: &PatternKind<In>,
    results: &mut Results<Out>,
    folder: &mut F,
) -> Result<PatternKind<Out>, F::Error>
where
    In: TreeBuilder,
    Out: TreeBuilder,
    F: Folder<In, Out>,
{
    Ok(match kind {
        PatternKind::Wildcard => PatternKind::Wildcard,
        PatternKind::None => PatternKind::None,
        PatternKind::Binding(name) => {
            PatternKind::Binding(folder.output_builder().alloc_str(name.as_ref()))
        }
        PatternKind::Literal(literal) => {
            PatternKind::Literal(rebuild_literal(literal, results, folder)?)
        }
        PatternKind::Some(_) => PatternKind::Some(take_one!(results.patterns)),
    })
}

fn rebuild_type_expr<In, Out, F>(
    kind: &TypeExprKind<In>,
    results: &mut Results<Out>,
    folder: &mut F,
) -> Result<TypeExprKind<Out>, F::Error>
where
    In: TreeBuilder,
    Out: TreeBuilder,
    F: Folder<In, Out>,
{
    let out = folder.output_builder().clone();
    Ok(match kind {
        TypeExprKind::Path(path) => TypeExprKind::Path(out.alloc_str(path.as_ref())),
        TypeExprKind::Parametrized { path, params } => TypeExprKind::Parametrized {
            path: out.alloc_str(path.as_ref()),
            params: out.alloc_list(take!(results.type_exprs, params.len()).collect::<Vec<_>>()),
        },
        TypeExprKind::Record(fields) => TypeExprKind::Record(
            out.alloc_list(take!(results.type_fields, fields.len()).collect::<Vec<_>>()),
        ),
    })
}

/// Two children in source order. `pop` yields them backwards.
fn take_two<T>(stack: &mut Vec<T>) -> (T, T) {
    let second = stack.pop().expect("driver bug: result stack underflow");
    let first = stack.pop().expect("driver bug: result stack underflow");
    (first, second)
}
