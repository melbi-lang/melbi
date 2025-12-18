//! Tests for the generic Fold trait and TypeFolder convenience trait.

use bumpalo::Bump;
use hashbrown::{HashMap, HashSet};
use melbi_types::{
    core::traversal::{drive_fold, fold_type, Fold, FoldStep, TypeFolder},
    ty, ArenaBuilder, BoxBuilder, Scalar, Ty, TyBuilder, TyKind,
};

// ============================================================================
// Identity Fold - rebuilds the same type
// ============================================================================

struct IdentityFolder;

impl<B: TyBuilder> TypeFolder<B> for IdentityFolder {
    fn fold_ty(&mut self, _b_in: &B, _b_out: &B, _ty: &Ty<B>) -> FoldStep<B, Ty<B>> {
        FoldStep::Recurse
    }
}

#[test]
fn test_identity_fold_scalar() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    let result = fold_type(&b, &b, ty!(b, Int), &mut IdentityFolder);
    assert_eq!(result, ty!(b, Int));
}

#[test]
fn test_identity_fold_array() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    let result = fold_type(&b, &b, ty!(b, Array[Int]), &mut IdentityFolder);
    assert_eq!(result, ty!(b, Array[Int]));
}

#[test]
fn test_identity_fold_map() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    let result = fold_type(&b, &b, ty!(b, Map[Str, Int]), &mut IdentityFolder);
    assert_eq!(result, ty!(b, Map[Str, Int]));
}

#[test]
fn test_identity_fold_nested() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    let result = fold_type(&b, &b, ty!(b, Array[Map[Str, Int]]), &mut IdentityFolder);
    assert_eq!(result, ty!(b, Array[Map[Str, Int]]));
}

// ============================================================================
// Type Substitution - replaces TypeVars with concrete types
// ============================================================================

struct Substitution<'a, B: TyBuilder> {
    mapping: &'a HashMap<u16, Ty<B>>,
}

impl<'a, B: TyBuilder> TypeFolder<B> for Substitution<'a, B> {
    fn fold_ty(&mut self, _b_in: &B, _b_out: &B, ty: &Ty<B>) -> FoldStep<B, Ty<B>> {
        if let TyKind::TypeVar(id) = ty.kind() {
            if let Some(replacement) = self.mapping.get(id) {
                // Use Replace to continue traversal into the replacement
                return FoldStep::Replace(replacement.clone());
            }
        }
        FoldStep::Recurse
    }
}

#[test]
fn test_substitution_simple() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    // TypeVar(0) -> Int
    let var0 = TyKind::TypeVar(0).alloc(&b);
    let mut mapping = HashMap::new();
    mapping.insert(0, ty!(b, Int));

    let result = fold_type(&b, &b, var0, &mut Substitution { mapping: &mapping });
    assert_eq!(result, ty!(b, Int));
}

#[test]
fn test_substitution_in_array() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    // Array[TypeVar(0)] with {0 -> Str} => Array[Str]
    let var0 = TyKind::TypeVar(0).alloc(&b);
    let arr_var0 = TyKind::Array(var0).alloc(&b);
    let mut mapping = HashMap::new();
    mapping.insert(0, ty!(b, Str));

    let result = fold_type(&b, &b, arr_var0, &mut Substitution { mapping: &mapping });
    assert_eq!(result, ty!(b, Array[Str]));
}

#[test]
fn test_substitution_chained() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    // TypeVar(0) with {0 -> Array[TypeVar(1)], 1 -> Bool}
    // Should resolve to Array[Bool]
    let var0 = TyKind::TypeVar(0).alloc(&b);
    let var1 = TyKind::TypeVar(1).alloc(&b);
    let arr_var1 = TyKind::Array(var1).alloc(&b);

    let mut mapping = HashMap::new();
    mapping.insert(0, arr_var1); // 0 -> Array[TypeVar(1)]
    mapping.insert(1, ty!(b, Bool)); // 1 -> Bool

    let result = fold_type(&b, &b, var0, &mut Substitution { mapping: &mapping });
    assert_eq!(result, ty!(b, Array[Bool]));
}

#[test]
fn test_substitution_preserves_unbound() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    // Map[TypeVar(0), TypeVar(1)] with {0 -> Int}
    // Should become Map[Int, TypeVar(1)]
    let var0 = TyKind::TypeVar(0).alloc(&b);
    let var1 = TyKind::TypeVar(1).alloc(&b);
    let map_ty = TyKind::Map(var0, var1).alloc(&b);

    let mut mapping = HashMap::new();
    mapping.insert(0, ty!(b, Int));

    let result = fold_type(&b, &b, map_ty, &mut Substitution { mapping: &mapping });

    // TypeVar(1) should be preserved
    let expected = TyKind::Map(ty!(b, Int), var1).alloc(&b);
    assert_eq!(result, expected);
}

// ============================================================================
// Done Early Exit - skip traversal when not needed
// ============================================================================

struct CountingFolder {
    visit_count: usize,
}

impl<B: TyBuilder> Fold<B> for CountingFolder {
    type Output = ();
    type Error = ();

    fn visit(&mut self, _builder: &B, _ty: &Ty<B>) -> Result<FoldStep<B, ()>, ()> {
        self.visit_count += 1;
        Ok(FoldStep::Recurse)
    }

    fn combine(
        &mut self,
        _builder: &B,
        _ty: &Ty<B>,
        _children: impl ExactSizeIterator<Item = ()> + DoubleEndedIterator,
    ) -> Result<(), ()> {
        Ok(())
    }
}

#[test]
fn test_visit_counts_all_nodes() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    // Array[Map[Int, Str]] has 4 nodes: Array, Map, Int, Str
    let mut folder = CountingFolder { visit_count: 0 };
    drive_fold(&b, ty!(b, Array[Map[Int, Str]]), &mut folder).unwrap();
    assert_eq!(folder.visit_count, 4);
}

struct EarlyExitFolder;

impl<B: TyBuilder> Fold<B> for EarlyExitFolder {
    type Output = usize;
    type Error = ();

    fn visit(&mut self, _builder: &B, ty: &Ty<B>) -> Result<FoldStep<B, usize>, ()> {
        // Early exit on scalars
        if matches!(ty.kind(), TyKind::Scalar(_)) {
            Ok(FoldStep::Done(1))
        } else {
            Ok(FoldStep::Recurse)
        }
    }

    fn combine(
        &mut self,
        _builder: &B,
        _ty: &Ty<B>,
        children: impl ExactSizeIterator<Item = usize> + DoubleEndedIterator,
    ) -> Result<usize, ()> {
        // Sum children and add 1 for this node
        Ok(children.sum::<usize>() + 1)
    }
}

#[test]
fn test_early_exit_done() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    // Array[Map[Int, Str]] should count: Array(1) + Map(1) + Int(1) + Str(1) = 4
    let result = drive_fold(&b, ty!(b, Array[Map[Int, Str]]), EarlyExitFolder).unwrap();
    assert_eq!(result, 4);
}

// ============================================================================
// Collect Type Variables
// ============================================================================

struct CollectTypeVars {
    vars: HashSet<u16>,
}

impl<B: TyBuilder> Fold<B> for CollectTypeVars {
    type Output = ();
    type Error = ();

    fn visit(&mut self, _builder: &B, ty: &Ty<B>) -> Result<FoldStep<B, ()>, ()> {
        if let TyKind::TypeVar(id) = ty.kind() {
            self.vars.insert(*id);
        }
        Ok(FoldStep::Recurse)
    }

    fn combine(
        &mut self,
        _builder: &B,
        _ty: &Ty<B>,
        _children: impl ExactSizeIterator<Item = ()> + DoubleEndedIterator,
    ) -> Result<(), ()> {
        Ok(())
    }
}

#[test]
fn test_collect_type_vars_none() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    let mut collector = CollectTypeVars {
        vars: HashSet::new(),
    };
    drive_fold(&b, ty!(b, Array[Int]), &mut collector).unwrap();
    assert!(collector.vars.is_empty());
}

#[test]
fn test_collect_type_vars_simple() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    let mut collector = CollectTypeVars {
        vars: HashSet::new(),
    };
    drive_fold(&b, ty!(b, [a, x] => Map[a, x]), &mut collector).unwrap();

    assert_eq!(collector.vars.len(), 2);
    assert!(collector.vars.contains(&0));
    assert!(collector.vars.contains(&1));
}

#[test]
fn test_collect_type_vars_nested() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    // Array[Map[TypeVar(0), Array[TypeVar(1)]]]
    let mut collector = CollectTypeVars {
        vars: HashSet::new(),
    };
    drive_fold(&b, ty!(b, [a, x] => Array[Map[a, Array[x]]]), &mut collector).unwrap();

    assert_eq!(collector.vars.len(), 2);
    assert!(collector.vars.contains(&0));
    assert!(collector.vars.contains(&1));
}

#[test]
fn test_collect_type_vars_duplicates() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    // Map[TypeVar(0), TypeVar(0)] - same var twice
    let mut collector = CollectTypeVars {
        vars: HashSet::new(),
    };
    drive_fold(&b, ty!(b, [a] => Map[a, a]), &mut collector).unwrap();

    assert_eq!(collector.vars.len(), 1);
    assert!(collector.vars.contains(&0));
}

// ============================================================================
// Cross-Builder Conversion: BoxBuilder -> ArenaBuilder
// ============================================================================

struct BoxToArenaFolder;

impl<'arena> TypeFolder<BoxBuilder, ArenaBuilder<'arena>> for BoxToArenaFolder {
    fn fold_ty(
        &mut self,
        _b_in: &BoxBuilder,
        _b_out: &ArenaBuilder<'arena>,
        _ty: &Ty<BoxBuilder>,
    ) -> FoldStep<BoxBuilder, Ty<ArenaBuilder<'arena>>> {
        // Just recurse - combine will rebuild in the new builder
        FoldStep::Recurse
    }
}

#[test]
fn test_box_to_arena_scalar() {
    let b_box = BoxBuilder;
    let arena = Bump::new();
    let b_arena = ArenaBuilder::new(&arena);

    let result = fold_type(&b_box, &b_arena, ty!(b_box, Int), &mut BoxToArenaFolder);
    assert_eq!(result, ty!(b_arena, Int));
}

#[test]
fn test_box_to_arena_array() {
    let b_box = BoxBuilder;
    let arena = Bump::new();
    let b_arena = ArenaBuilder::new(&arena);

    let result = fold_type(&b_box, &b_arena, ty!(b_box, Array[Int]), &mut BoxToArenaFolder);
    assert_eq!(result, ty!(b_arena, Array[Int]));
}

#[test]
fn test_box_to_arena_map() {
    let b_box = BoxBuilder;
    let arena = Bump::new();
    let b_arena = ArenaBuilder::new(&arena);

    let result = fold_type(&b_box, &b_arena, ty!(b_box, Map[Str, Int]), &mut BoxToArenaFolder);
    assert_eq!(result, ty!(b_arena, Map[Str, Int]));
}

#[test]
fn test_box_to_arena_nested() {
    let b_box = BoxBuilder;
    let arena = Bump::new();
    let b_arena = ArenaBuilder::new(&arena);

    let result = fold_type(
        &b_box,
        &b_arena,
        ty!(b_box, Array[Map[Str, Array[Int]]]),
        &mut BoxToArenaFolder,
    );
    assert_eq!(result, ty!(b_arena, Array[Map[Str, Array[Int]]]));
}

#[test]
fn test_box_to_arena_type_var() {
    let b_box = BoxBuilder;
    let arena = Bump::new();
    let b_arena = ArenaBuilder::new(&arena);

    let result = fold_type(
        &b_box,
        &b_arena,
        ty!(b_box, [a] => Array[a]),
        &mut BoxToArenaFolder,
    );
    assert_eq!(result, ty!(b_arena, [a] => Array[a]));
}

// ============================================================================
// Cross-Builder Conversion: ArenaBuilder -> BoxBuilder
// ============================================================================

struct ArenaToBoxFolder;

impl<'arena> TypeFolder<ArenaBuilder<'arena>, BoxBuilder> for ArenaToBoxFolder {
    fn fold_ty(
        &mut self,
        _b_in: &ArenaBuilder<'arena>,
        _b_out: &BoxBuilder,
        _ty: &Ty<ArenaBuilder<'arena>>,
    ) -> FoldStep<ArenaBuilder<'arena>, Ty<BoxBuilder>> {
        FoldStep::Recurse
    }
}

#[test]
fn test_arena_to_box_roundtrip() {
    let b_box = BoxBuilder;
    let arena = Bump::new();
    let b_arena = ArenaBuilder::new(&arena);

    // Box -> Arena -> Box roundtrip
    let original = ty!(b_box, Array[Map[Str, Int]]);
    let in_arena = fold_type(&b_box, &b_arena, original, &mut BoxToArenaFolder);
    let back_to_box = fold_type(&b_arena, &b_box, in_arena, &mut ArenaToBoxFolder);

    // BoxBuilder uses structural equality, so this compares structure
    assert_eq!(back_to_box, ty!(b_box, Array[Map[Str, Int]]));
}

// ============================================================================
// Error Propagation
// ============================================================================

struct FailingFolder {
    fail_on_var: u16,
}

impl<B: TyBuilder> Fold<B> for FailingFolder {
    type Output = ();
    type Error = String;

    fn visit(&mut self, _builder: &B, ty: &Ty<B>) -> Result<FoldStep<B, ()>, String> {
        if let TyKind::TypeVar(id) = ty.kind() {
            if *id == self.fail_on_var {
                return Err(format!("Failed on TypeVar({})", id));
            }
        }
        Ok(FoldStep::Recurse)
    }

    fn combine(
        &mut self,
        _builder: &B,
        _ty: &Ty<B>,
        _children: impl ExactSizeIterator<Item = ()> + DoubleEndedIterator,
    ) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn test_error_propagation() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    let result = drive_fold(&b, ty!(b, [a, x] => Map[a, x]), FailingFolder { fail_on_var: 1 });
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Failed on TypeVar(1)");
}

#[test]
fn test_error_no_propagation_when_ok() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    // No TypeVar(99) in the tree, so should succeed
    let result = drive_fold(&b, ty!(b, Array[Int]), FailingFolder { fail_on_var: 99 });
    assert!(result.is_ok());
}

// ============================================================================
// Leaf Types (TypeVar, Scalar, Symbol)
// ============================================================================

#[test]
fn test_fold_type_var() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    let input = TyKind::TypeVar(42).alloc(&b);
    let result = fold_type(&b, &b, input, &mut IdentityFolder);
    assert_eq!(result.kind(), &TyKind::TypeVar(42));
}

#[test]
fn test_fold_all_scalars() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    for scalar in [
        Scalar::Bool,
        Scalar::Int,
        Scalar::Float,
        Scalar::Str,
        Scalar::Bytes,
    ] {
        let ty = TyKind::Scalar(scalar).alloc(&b);
        let result = fold_type(&b, &b, ty, &mut IdentityFolder);
        assert_eq!(result.kind(), &TyKind::Scalar(scalar));
    }
}

// ============================================================================
// Child Order Verification
// ============================================================================

struct ChildOrderCollector {
    order: Vec<&'static str>,
}

impl<B: TyBuilder> Fold<B> for ChildOrderCollector {
    type Output = &'static str;
    type Error = ();

    fn visit(&mut self, _builder: &B, ty: &Ty<B>) -> Result<FoldStep<B, &'static str>, ()> {
        let label = match ty.kind() {
            TyKind::Scalar(Scalar::Int) => "int",
            TyKind::Scalar(Scalar::Str) => "str",
            TyKind::Scalar(Scalar::Bool) => "bool",
            TyKind::Array(_) => return Ok(FoldStep::Recurse),
            TyKind::Map(_, _) => return Ok(FoldStep::Recurse),
            _ => "other",
        };
        self.order.push(label);
        Ok(FoldStep::Done(label))
    }

    fn combine(
        &mut self,
        _builder: &B,
        ty: &Ty<B>,
        children: impl ExactSizeIterator<Item = &'static str> + DoubleEndedIterator,
    ) -> Result<&'static str, ()> {
        let collected: Vec<_> = children.collect();
        match ty.kind() {
            TyKind::Map(_, _) => {
                assert_eq!(collected.len(), 2, "Map should have 2 children");
                Ok("map")
            }
            TyKind::Array(_) => {
                assert_eq!(collected.len(), 1, "Array should have 1 child");
                Ok("array")
            }
            _ => Ok("other"),
        }
    }
}

#[test]
fn test_map_child_order() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    // Map[Int, Str] - children should be visited in order: key (Int), value (Str)
    let mut collector = ChildOrderCollector { order: vec![] };
    drive_fold(&b, ty!(b, Map[Int, Str]), &mut collector).unwrap();

    // Children are visited in definition order: key first, then value
    assert_eq!(collector.order, vec!["int", "str"]);
}

#[test]
fn test_nested_child_order() {
    let arena = Bump::new();
    let b = ArenaBuilder::new(&arena);

    // Array[Map[Int, Bool]] - should visit: Int, Bool (leaves in order)
    let mut collector = ChildOrderCollector { order: vec![] };
    drive_fold(&b, ty!(b, Array[Map[Int, Bool]]), &mut collector).unwrap();

    assert_eq!(collector.order, vec!["int", "bool"]);
}
