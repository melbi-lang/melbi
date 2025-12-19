use crate::core::{builder::TyBuilder, kind::TyKind, ty::Ty};

/// A generic trait for traversing a type while modifying a context.
/// This is supposed to be implemented on TyKind<B>.
pub trait Visit<B: TyBuilder, C> {
    /// Visit the node.
    /// The implementation on `Ty<B>` handles the data (flags) and forwards
    /// execution to the Kind.
    fn visit(&self, builder: &B, ctx: &mut C);

    /// Walk over children nodes calling `visit` recursively.
    /// This provides the default traversal behavior that implementations
    /// can call to recurse into children after processing the current node.
    fn walk(&self, builder: &B, ctx: &mut C);
}

impl<B, C> Visit<B, C> for Ty<B>
where
    B: TyBuilder,
    TyKind<B>: Visit<B, C>,
{
    /// This implementation just forwards to TyKind for convenience.
    fn visit(&self, builder: &B, ctx: &mut C) {
        self.kind().visit(builder, ctx)
    }

    /// This arbitrarily forwards the call to a visit on TyKind.
    fn walk(&self, builder: &B, ctx: &mut C) {
        self.kind().visit(builder, ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builders::BoxBuilder;
    use crate::core::kind::Scalar;

    struct IntCounterCtx {
        count: usize,
    }

    impl Visit<BoxBuilder, IntCounterCtx> for TyKind<BoxBuilder> {
        fn visit(&self, builder: &BoxBuilder, ctx: &mut IntCounterCtx) {
            match self {
                TyKind::Scalar(Scalar::Int) => {
                    ctx.count += 1;
                }
                _ => self.walk(builder, ctx),
            };
        }

        fn walk(&self, builder: &BoxBuilder, ctx: &mut IntCounterCtx) {
            for child in self.iter_children() {
                child.visit(builder, ctx);
            }
        }
    }

    #[test]
    fn test_count_int_two() {
        let builder = BoxBuilder::new();
        let map = TyKind::Map(
            TyKind::Scalar(Scalar::Int).alloc(&builder),
            TyKind::Scalar(Scalar::Int).alloc(&builder),
        )
        .alloc(&builder);

        let mut ctx = IntCounterCtx { count: 0 };
        map.visit(&builder, &mut ctx);
        assert_eq!(ctx.count, 2);
    }

    #[test]
    fn test_count_int_one() {
        let builder = BoxBuilder::new();
        let map = TyKind::Map(
            TyKind::Array(TyKind::Scalar(Scalar::Int).alloc(&builder)).alloc(&builder),
            TyKind::Map(
                TyKind::Scalar(Scalar::Str).alloc(&builder),
                TyKind::Scalar(Scalar::Float).alloc(&builder),
            )
            .alloc(&builder),
        )
        .alloc(&builder);

        let mut ctx = IntCounterCtx { count: 0 };
        map.visit(&builder, &mut ctx);
        assert_eq!(ctx.count, 1);
    }
}
