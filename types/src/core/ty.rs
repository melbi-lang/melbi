use super::builder::TyBuilder;
use super::flags::TyFlags;
use super::kind::TyKind;

/// Lightweight wrapper around the builder's representation.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Ty<B: TyBuilder>(B::TyHandle);

impl<B: TyBuilder> Ty<B> {
    pub fn new(builder: &B, node: TyNode<B>) -> Self {
        Self(builder.alloc(node))
    }

    pub fn handle(&self) -> &B::TyHandle {
        &self.0
    }

    pub fn node(&self) -> &TyNode<B> {
        self.handle().as_ref()
    }
}

// Implement Copy when TyHandle is Copy (e.g., for ArenaBuilder)
impl<B: TyBuilder> Copy for Ty<B> where B::TyHandle: Copy {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TyNode<B: TyBuilder>(TyFlags, TyKind<B>);

impl<B: TyBuilder> TyNode<B> {
    pub fn new(kind: TyKind<B>) -> Self {
        let flags = kind.compute_flags();
        Self(flags, kind)
    }
}

impl<B: TyBuilder> TyNode<B> {
    pub fn flags(&self) -> TyFlags {
        self.0
    }

    pub fn kind(&self) -> &TyKind<B> {
        &self.1
    }
}

impl<B: TyBuilder> AsRef<TyNode<B>> for TyNode<B> {
    fn as_ref(&self) -> &TyNode<B> {
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident<B: TyBuilder>(B::IdentHandle);

impl<B: TyBuilder> Ident<B> {
    pub fn new(builder: &B, name: impl AsRef<str>) -> Self {
        Self(builder.alloc_ident(name))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

// Implement Copy when IdentHandle is Copy (e.g., for ArenaBuilder)
impl<B: TyBuilder> Copy for Ident<B> where B::IdentHandle: Copy {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TyList<B: TyBuilder>(B::TyListHandle);

impl<B: TyBuilder> TyList<B> {
    pub fn from_iter(
        builder: &B,
        iter: impl IntoIterator<Item = Ty<B>, IntoIter: ExactSizeIterator>,
    ) -> Self {
        Self(builder.alloc_ty_list(iter))
    }

    pub fn iter(&self) -> core::slice::Iter<'_, Ty<B>> {
        self.0.iter()
    }
}

impl<'a, B: TyBuilder> IntoIterator for &'a TyList<B> {
    type Item = &'a Ty<B>;
    type IntoIter = core::slice::Iter<'a, Ty<B>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdentList<B: TyBuilder>(B::IdentListHandle);

impl<B: TyBuilder> IdentList<B> {
    pub fn from_iter(
        builder: &B,
        iter: impl IntoIterator<Item = Ident<B>, IntoIter: ExactSizeIterator>,
    ) -> Self {
        Self(builder.alloc_ident_list(iter))
    }

    pub fn iter(&self) -> core::slice::Iter<'_, Ident<B>> {
        self.0.iter()
    }
}

impl<'a, B: TyBuilder> IntoIterator for &'a IdentList<B> {
    type Item = &'a Ident<B>;
    type IntoIter = core::slice::Iter<'a, Ident<B>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FieldList<B: TyBuilder>(B::FieldListHandle);

impl<B: TyBuilder> FieldList<B> {
    pub fn from_iter(
        builder: &B,
        iter: impl IntoIterator<Item = (Ident<B>, Ty<B>), IntoIter: ExactSizeIterator>,
    ) -> Self {
        Self(builder.alloc_field_list(iter))
    }

    pub fn iter(&self) -> core::slice::Iter<'_, (Ident<B>, Ty<B>)> {
        self.0.iter()
    }
}

impl<'a, B: TyBuilder> IntoIterator for &'a FieldList<B> {
    type Item = &'a (Ident<B>, Ty<B>);
    type IntoIter = core::slice::Iter<'a, (Ident<B>, Ty<B>)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
