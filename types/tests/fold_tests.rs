//! Tests for the generic Fold trait and TypeFolder convenience trait.

use bumpalo::Bump;
use hashbrown::{HashMap, HashSet};
use melbi_types::{
    ArenaBuilder, BoxBuilder, Scalar, Ty, TyBuilder, TyKind,
    core::traversal::{Fold, FoldStep, TypeFolder, drive_fold, fold_type},
};

// ============================================================================
// Test Helpers
// ============================================================================

/// Helper to create an ArenaBuilder
fn with_arena<R>(f: impl FnOnce(&ArenaBuilder) -> R) -> R {
    let arena = Bump::new();
    let builder = ArenaBuilder::new(&arena);
    f(&builder)
}

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
    with_arena(|b| {
        let int_ty = TyKind::Scalar(Scalar::Int).alloc(b);
        let result = fold_type(b, b, int_ty, &mut IdentityFolder);
        assert_eq!(result.kind(), &TyKind::Scalar(Scalar::Int));
    });
}

#[test]
fn test_identity_fold_array() {
    with_arena(|b| {
        let int_ty = TyKind::Scalar(Scalar::Int).alloc(b);
        let arr_ty = TyKind::Array(int_ty).alloc(b);
        let result = fold_type(b, b, arr_ty, &mut IdentityFolder);

        match result.kind() {
            TyKind::Array(elem) => assert_eq!(elem.kind(), &TyKind::Scalar(Scalar::Int)),
            _ => panic!("Expected Array"),
        }
    });
}

#[test]
fn test_identity_fold_map() {
    with_arena(|b| {
        let str_ty = TyKind::Scalar(Scalar::Str).alloc(b);
        let int_ty = TyKind::Scalar(Scalar::Int).alloc(b);
        let map_ty = TyKind::Map(str_ty, int_ty).alloc(b);
        let result = fold_type(b, b, map_ty, &mut IdentityFolder);

        match result.kind() {
            TyKind::Map(k, v) => {
                assert_eq!(k.kind(), &TyKind::Scalar(Scalar::Str));
                assert_eq!(v.kind(), &TyKind::Scalar(Scalar::Int));
            }
            _ => panic!("Expected Map"),
        }
    });
}

#[test]
fn test_identity_fold_nested() {
    with_arena(|b| {
        // Array[Map[Str, Int]]
        let str_ty = TyKind::Scalar(Scalar::Str).alloc(b);
        let int_ty = TyKind::Scalar(Scalar::Int).alloc(b);
        let map_ty = TyKind::Map(str_ty, int_ty).alloc(b);
        let arr_ty = TyKind::Array(map_ty).alloc(b);
        let result = fold_type(b, b, arr_ty, &mut IdentityFolder);

        match result.kind() {
            TyKind::Array(elem) => match elem.kind() {
                TyKind::Map(k, v) => {
                    assert_eq!(k.kind(), &TyKind::Scalar(Scalar::Str));
                    assert_eq!(v.kind(), &TyKind::Scalar(Scalar::Int));
                }
                _ => panic!("Expected Map inside Array"),
            },
            _ => panic!("Expected Array"),
        }
    });
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
    with_arena(|b| {
        // TypeVar(0) -> Int
        let var0 = TyKind::TypeVar(0).alloc(b);
        let int_ty = TyKind::Scalar(Scalar::Int).alloc(b);

        let mut mapping = HashMap::new();
        mapping.insert(0, int_ty);

        let result = fold_type(b, b, var0, &mut Substitution { mapping: &mapping });
        assert_eq!(result.kind(), &TyKind::Scalar(Scalar::Int));
    });
}

#[test]
fn test_substitution_in_array() {
    with_arena(|b| {
        // Array[TypeVar(0)] with {0 -> Str} => Array[Str]
        let var0 = TyKind::TypeVar(0).alloc(b);
        let arr_ty = TyKind::Array(var0).alloc(b);
        let str_ty = TyKind::Scalar(Scalar::Str).alloc(b);

        let mut mapping = HashMap::new();
        mapping.insert(0, str_ty);

        let result = fold_type(b, b, arr_ty, &mut Substitution { mapping: &mapping });

        match result.kind() {
            TyKind::Array(elem) => assert_eq!(elem.kind(), &TyKind::Scalar(Scalar::Str)),
            _ => panic!("Expected Array"),
        }
    });
}

#[test]
fn test_substitution_chained() {
    with_arena(|b| {
        // TypeVar(0) with {0 -> Array[TypeVar(1)], 1 -> Bool}
        // Should resolve to Array[Bool]
        let var0 = TyKind::TypeVar(0).alloc(b);
        let var1 = TyKind::TypeVar(1).alloc(b);
        let arr_var1 = TyKind::Array(var1).alloc(b);
        let bool_ty = TyKind::Scalar(Scalar::Bool).alloc(b);

        let mut mapping = HashMap::new();
        mapping.insert(0, arr_var1);
        mapping.insert(1, bool_ty);

        let result = fold_type(b, b, var0, &mut Substitution { mapping: &mapping });

        match result.kind() {
            TyKind::Array(elem) => assert_eq!(elem.kind(), &TyKind::Scalar(Scalar::Bool)),
            _ => panic!("Expected Array[Bool], got {:?}", result.kind()),
        }
    });
}

#[test]
fn test_substitution_preserves_unbound() {
    with_arena(|b| {
        // Map[TypeVar(0), TypeVar(1)] with {0 -> Int}
        // Should become Map[Int, TypeVar(1)]
        let var0 = TyKind::TypeVar(0).alloc(b);
        let var1 = TyKind::TypeVar(1).alloc(b);
        let map_ty = TyKind::Map(var0, var1).alloc(b);
        let int_ty = TyKind::Scalar(Scalar::Int).alloc(b);

        let mut mapping = HashMap::new();
        mapping.insert(0, int_ty);

        let result = fold_type(b, b, map_ty, &mut Substitution { mapping: &mapping });

        match result.kind() {
            TyKind::Map(k, v) => {
                assert_eq!(k.kind(), &TyKind::Scalar(Scalar::Int));
                assert_eq!(v.kind(), &TyKind::TypeVar(1));
            }
            _ => panic!("Expected Map"),
        }
    });
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
    with_arena(|b| {
        // Array[Map[Int, Str]] has 4 nodes: Array, Map, Int, Str
        let int_ty = TyKind::Scalar(Scalar::Int).alloc(b);
        let str_ty = TyKind::Scalar(Scalar::Str).alloc(b);
        let map_ty = TyKind::Map(int_ty, str_ty).alloc(b);
        let arr_ty = TyKind::Array(map_ty).alloc(b);

        let mut folder = CountingFolder { visit_count: 0 };
        drive_fold(b, arr_ty, &mut folder).unwrap();

        assert_eq!(folder.visit_count, 4);
    });
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
    with_arena(|b| {
        // Array[Map[Int, Str]] should count: Array(1) + Map(1) + Int(1) + Str(1) = 4
        let int_ty = TyKind::Scalar(Scalar::Int).alloc(b);
        let str_ty = TyKind::Scalar(Scalar::Str).alloc(b);
        let map_ty = TyKind::Map(int_ty, str_ty).alloc(b);
        let arr_ty = TyKind::Array(map_ty).alloc(b);

        let result = drive_fold(b, arr_ty, EarlyExitFolder).unwrap();
        assert_eq!(result, 4);
    });
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
    with_arena(|b| {
        let int_ty = TyKind::Scalar(Scalar::Int).alloc(b);
        let arr_ty = TyKind::Array(int_ty).alloc(b);

        let mut collector = CollectTypeVars {
            vars: HashSet::new(),
        };
        drive_fold(b, arr_ty, &mut collector).unwrap();

        assert!(collector.vars.is_empty());
    });
}

#[test]
fn test_collect_type_vars_simple() {
    with_arena(|b| {
        let var0 = TyKind::TypeVar(0).alloc(b);
        let var1 = TyKind::TypeVar(1).alloc(b);
        let map_ty = TyKind::Map(var0, var1).alloc(b);

        let mut collector = CollectTypeVars {
            vars: HashSet::new(),
        };
        drive_fold(b, map_ty, &mut collector).unwrap();

        assert_eq!(collector.vars.len(), 2);
        assert!(collector.vars.contains(&0));
        assert!(collector.vars.contains(&1));
    });
}

#[test]
fn test_collect_type_vars_nested() {
    with_arena(|b| {
        // Array[Map[TypeVar(0), Array[TypeVar(1)]]]
        let var0 = TyKind::TypeVar(0).alloc(b);
        let var1 = TyKind::TypeVar(1).alloc(b);
        let inner_arr = TyKind::Array(var1).alloc(b);
        let map_ty = TyKind::Map(var0, inner_arr).alloc(b);
        let outer_arr = TyKind::Array(map_ty).alloc(b);

        let mut collector = CollectTypeVars {
            vars: HashSet::new(),
        };
        drive_fold(b, outer_arr, &mut collector).unwrap();

        assert_eq!(collector.vars.len(), 2);
        assert!(collector.vars.contains(&0));
        assert!(collector.vars.contains(&1));
    });
}

#[test]
fn test_collect_type_vars_duplicates() {
    with_arena(|b| {
        // Map[TypeVar(0), TypeVar(0)] - same var twice
        let var0 = TyKind::TypeVar(0).alloc(b);
        let map_ty = TyKind::Map(var0, var0).alloc(b);

        let mut collector = CollectTypeVars {
            vars: HashSet::new(),
        };
        drive_fold(b, map_ty, &mut collector).unwrap();

        assert_eq!(collector.vars.len(), 1);
        assert!(collector.vars.contains(&0));
    });
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
    let box_builder = BoxBuilder;
    let box_int = TyKind::Scalar(Scalar::Int).alloc(&box_builder);

    let arena = Bump::new();
    let arena_builder = ArenaBuilder::new(&arena);

    let arena_int = fold_type(&box_builder, &arena_builder, box_int, &mut BoxToArenaFolder);
    assert_eq!(arena_int.kind(), &TyKind::Scalar(Scalar::Int));
}

#[test]
fn test_box_to_arena_array() {
    let box_builder = BoxBuilder;
    let box_int = TyKind::Scalar(Scalar::Int).alloc(&box_builder);
    let box_arr = TyKind::Array(box_int).alloc(&box_builder);

    let arena = Bump::new();
    let arena_builder = ArenaBuilder::new(&arena);

    let arena_arr = fold_type(&box_builder, &arena_builder, box_arr, &mut BoxToArenaFolder);

    match arena_arr.kind() {
        TyKind::Array(elem) => assert_eq!(elem.kind(), &TyKind::Scalar(Scalar::Int)),
        _ => panic!("Expected Array"),
    }
}

#[test]
fn test_box_to_arena_map() {
    let box_builder = BoxBuilder;
    let box_str = TyKind::Scalar(Scalar::Str).alloc(&box_builder);
    let box_int = TyKind::Scalar(Scalar::Int).alloc(&box_builder);
    let box_map = TyKind::Map(box_str, box_int).alloc(&box_builder);

    let arena = Bump::new();
    let arena_builder = ArenaBuilder::new(&arena);

    let arena_map = fold_type(&box_builder, &arena_builder, box_map, &mut BoxToArenaFolder);

    match arena_map.kind() {
        TyKind::Map(k, v) => {
            assert_eq!(k.kind(), &TyKind::Scalar(Scalar::Str));
            assert_eq!(v.kind(), &TyKind::Scalar(Scalar::Int));
        }
        _ => panic!("Expected Map"),
    }
}

#[test]
fn test_box_to_arena_nested() {
    let box_builder = BoxBuilder;
    // Array[Map[Str, Array[Int]]]
    let box_int = TyKind::Scalar(Scalar::Int).alloc(&box_builder);
    let box_str = TyKind::Scalar(Scalar::Str).alloc(&box_builder);
    let box_inner_arr = TyKind::Array(box_int).alloc(&box_builder);
    let box_map = TyKind::Map(box_str, box_inner_arr).alloc(&box_builder);
    let box_outer_arr = TyKind::Array(box_map).alloc(&box_builder);

    let arena = Bump::new();
    let arena_builder = ArenaBuilder::new(&arena);

    let arena_result = fold_type(
        &box_builder,
        &arena_builder,
        box_outer_arr,
        &mut BoxToArenaFolder,
    );

    // Verify structure
    match arena_result.kind() {
        TyKind::Array(outer_elem) => match outer_elem.kind() {
            TyKind::Map(k, v) => {
                assert_eq!(k.kind(), &TyKind::Scalar(Scalar::Str));
                match v.kind() {
                    TyKind::Array(inner_elem) => {
                        assert_eq!(inner_elem.kind(), &TyKind::Scalar(Scalar::Int))
                    }
                    _ => panic!("Expected inner Array"),
                }
            }
            _ => panic!("Expected Map"),
        },
        _ => panic!("Expected outer Array"),
    }
}

#[test]
fn test_box_to_arena_type_var() {
    let box_builder = BoxBuilder;
    let box_var = TyKind::TypeVar(42).alloc(&box_builder);
    let box_arr = TyKind::Array(box_var).alloc(&box_builder);

    let arena = Bump::new();
    let arena_builder = ArenaBuilder::new(&arena);

    let arena_arr = fold_type(&box_builder, &arena_builder, box_arr, &mut BoxToArenaFolder);

    match arena_arr.kind() {
        TyKind::Array(elem) => assert_eq!(elem.kind(), &TyKind::TypeVar(42)),
        _ => panic!("Expected Array"),
    }
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
    // Create in BoxBuilder, convert to Arena, convert back to Box
    let box_builder = BoxBuilder;
    let original =
        TyKind::Array(TyKind::Scalar(Scalar::Int).alloc(&box_builder)).alloc(&box_builder);

    let arena = Bump::new();
    let arena_builder = ArenaBuilder::new(&arena);

    // Box -> Arena
    let arena_ty = fold_type(
        &box_builder,
        &arena_builder,
        original.clone(),
        &mut BoxToArenaFolder,
    );

    // Arena -> Box
    let back_to_box = fold_type(
        &arena_builder,
        &box_builder,
        arena_ty,
        &mut ArenaToBoxFolder,
    );

    // Verify structure is preserved
    match back_to_box.kind() {
        TyKind::Array(elem) => assert_eq!(elem.kind(), &TyKind::Scalar(Scalar::Int)),
        _ => panic!("Expected Array"),
    }
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
    with_arena(|b| {
        let var0 = TyKind::TypeVar(0).alloc(b);
        let var1 = TyKind::TypeVar(1).alloc(b);
        let map_ty = TyKind::Map(var0, var1).alloc(b);

        let result = drive_fold(b, map_ty, FailingFolder { fail_on_var: 1 });
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Failed on TypeVar(1)");
    });
}

#[test]
fn test_error_no_propagation_when_ok() {
    with_arena(|b| {
        let int_ty = TyKind::Scalar(Scalar::Int).alloc(b);
        let arr_ty = TyKind::Array(int_ty).alloc(b);

        // No TypeVar(99) in the tree, so should succeed
        let result = drive_fold(b, arr_ty, FailingFolder { fail_on_var: 99 });
        assert!(result.is_ok());
    });
}

// ============================================================================
// Leaf Types (TypeVar, Scalar, Symbol)
// ============================================================================

#[test]
fn test_fold_type_var() {
    with_arena(|b| {
        let var = TyKind::TypeVar(42).alloc(b);
        let result = fold_type(b, b, var, &mut IdentityFolder);
        assert_eq!(result.kind(), &TyKind::TypeVar(42));
    });
}

#[test]
fn test_fold_all_scalars() {
    with_arena(|b| {
        for scalar in [
            Scalar::Bool,
            Scalar::Int,
            Scalar::Float,
            Scalar::Str,
            Scalar::Bytes,
        ] {
            let ty = TyKind::Scalar(scalar).alloc(b);
            let result = fold_type(b, b, ty, &mut IdentityFolder);
            assert_eq!(result.kind(), &TyKind::Scalar(scalar));
        }
    });
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
    with_arena(|b| {
        // Map[Int, Str] - children should be visited in order: key (Int), value (Str)
        let int_ty = TyKind::Scalar(Scalar::Int).alloc(b);
        let str_ty = TyKind::Scalar(Scalar::Str).alloc(b);
        let map_ty = TyKind::Map(int_ty, str_ty).alloc(b);

        let mut collector = ChildOrderCollector { order: vec![] };
        drive_fold(b, map_ty, &mut collector).unwrap();

        // Children are visited in definition order: key first, then value
        assert_eq!(collector.order, vec!["int", "str"]);
    });
}

#[test]
fn test_nested_child_order() {
    with_arena(|b| {
        // Array[Map[Int, Bool]] - should visit: Int, Bool (leaves in order)
        let int_ty = TyKind::Scalar(Scalar::Int).alloc(b);
        let bool_ty = TyKind::Scalar(Scalar::Bool).alloc(b);
        let map_ty = TyKind::Map(int_ty, bool_ty).alloc(b);
        let arr_ty = TyKind::Array(map_ty).alloc(b);

        let mut collector = ChildOrderCollector { order: vec![] };
        drive_fold(b, arr_ty, &mut collector).unwrap();

        assert_eq!(collector.order, vec!["int", "bool"]);
    });
}
