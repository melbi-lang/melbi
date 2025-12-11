use crate::core::{
    kind::TyKind,
    ty::Ty,
    builder::TyBuilder,
};

/// A generic trait for traversing a type and producing a result.
pub trait Visit<B: TyBuilder, C> {
    /// Visit the node.
    /// The implementation on `Ty<B>` handles the data (flags) and forwards
    /// execution to the Kind.
    fn visit(&self, builder: &B, ctx: &mut C);

    fn walk(&self, builder: &B, ctx: &mut C);
}

impl<B, C> Visit<B, C> for Ty<B>
where
    B: TyBuilder,
    TyKind<B>: Visit<B, C>,
{
    fn visit(&self, builder: &B, ctx: &mut C) {
        let node = self.node();
        node.kind().visit(builder, ctx)
    }

    fn walk(&self, builder: &B, ctx: &mut C) {
        self.visit(builder, ctx);
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
            match self {
                TyKind::TypeVar(_) => {}
                TyKind::Scalar(_) => {}
                TyKind::Array(e) => e.visit(builder, ctx),
                TyKind::Map(k, v) => {
                    k.visit(builder, ctx);
                    v.visit(builder, ctx);
                }
                TyKind::Record(fields) => {
                    for (_, field_ty) in fields {
                        field_ty.visit(builder, ctx);
                    }
                }
                TyKind::Function { params, ret } => {
                    for param in params {
                        param.visit(builder, ctx);
                    }
                    ret.visit(builder, ctx);
                }
                TyKind::Symbol(_) => {}
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
