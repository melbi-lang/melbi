use core::hash;
use core::ops::Deref;

use super::builder::TyBuilder;
use super::flags::TyFlags;
use super::kind::TyKind;

/// Lightweight wrapper around the builder's type representation.
///
/// `Ty<B>` is the primary type handle used throughout the type system. It wraps
/// a builder-specific handle (`B::TyHandle`) and provides access to the underlying
/// [`TyNode`] containing the type's [`TyKind`] and [`TyFlags`].
///
/// Equality and hashing delegate to [`TyBuilder::ty_eq`] and [`TyBuilder::ty_hash`],
/// allowing builders to customize behavior (e.g., pointer-based for interning).
///
/// # Examples
///
/// ```
/// use melbi_types::builders::BoxBuilder;
/// use melbi_types::{Ty, TyKind, Scalar};
///
/// let builder = BoxBuilder::new();
///
/// // Create a scalar type
/// let int_ty = Ty::new(&builder, TyKind::Scalar(Scalar::Int));
/// assert!(matches!(int_ty.kind(), TyKind::Scalar(Scalar::Int)));
///
/// // Create an array type
/// let array_ty = Ty::new(&builder, TyKind::Array(int_ty.clone()));
/// assert!(matches!(array_ty.kind(), TyKind::Array(_)));
/// ```
#[derive(Clone, Debug)]
pub struct Ty<B: TyBuilder>(B::TyHandle);

impl<B: TyBuilder> PartialEq for Ty<B> {
    fn eq(&self, other: &Self) -> bool {
        B::ty_eq(self, other)
    }
}

impl<B: TyBuilder> Eq for Ty<B> {}

impl<B: TyBuilder> hash::Hash for Ty<B> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        B::ty_hash(self, state)
    }
}

impl<B: TyBuilder> Ty<B> {
    pub fn new(builder: &B, kind: TyKind<B>) -> Self {
        Self(builder.alloc(kind))
    }

    pub fn handle(&self) -> B::TyHandle {
        self.0.clone()
    }

    pub fn node(&self) -> &TyNode<B> {
        self.0.as_ref()
    }

    pub fn kind(&self) -> &TyKind<B> {
        self.node().kind()
    }

    pub fn flags(&self) -> TyFlags {
        self.node().flags()
    }
}

// Implement Copy when TyHandle is Copy (e.g., for ArenaBuilder)
impl<B: TyBuilder> Copy for Ty<B> where B::TyHandle: Copy {}

/// Immutable node containing a type's [`TyKind`] and computed [`TyFlags`].
///
/// `TyNode<B>` is the underlying storage for types, wrapped by [`Ty<B>`].
/// It computes flags once at construction time and provides access to the
/// type's kind.
///
/// # Hashing
///
/// `Hash` is implemented to hash only the kind (not flags, since they are
/// derived from the kind). This supports interning where structurally
/// identical types should hash equally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TyNode<B: TyBuilder>(TyFlags, TyKind<B>);

impl<B: TyBuilder> TyNode<B> {
    pub fn new(kind: TyKind<B>) -> Self {
        let flags = kind.compute_flags();
        Self(flags, kind)
    }

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

impl<B: TyBuilder> hash::Hash for TyNode<B> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        // Flags are computed from the kind, so we don't need to hash them.
        self.kind().hash(state);
    }
}

// === Ident ===

/// Identifier (interned string).
///
/// Equality delegates to `TyBuilder::ident_eq`, allowing builders to customize
/// behavior (e.g., pointer-based for interning).
#[derive(Debug, Clone)]
pub struct Ident<B: TyBuilder>(B::IdentHandle);

impl<B: TyBuilder> PartialEq for Ident<B> {
    fn eq(&self, other: &Self) -> bool {
        B::ident_eq(self, other)
    }
}

impl<B: TyBuilder> Eq for Ident<B> {}

impl<B: TyBuilder> hash::Hash for Ident<B> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        B::ident_hash(self, state)
    }
}

impl<B: TyBuilder> Ident<B> {
    pub fn new(builder: &B, name: impl AsRef<str>) -> Self {
        Self(builder.alloc_ident(name))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    /// Returns a reference to the underlying `IdentHandle`.
    ///
    /// This is useful for builders that need to access the handle directly,
    /// such as for interned comparison operations.
    pub fn handle(&self) -> &B::IdentHandle {
        &self.0
    }
}

// Implement Copy when IdentHandle is Copy (e.g., for ArenaBuilder)
impl<B: TyBuilder> Copy for Ident<B> where B::IdentHandle: Copy {}

impl<B: TyBuilder> Deref for Ident<B> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

// === TyList ===

/// An owned list of types, backed by the builder's storage.
///
/// `TyList<B>` is a thin wrapper around the builder's [`TyBuilder::TyListHandle`],
/// representing a sequence of [`Ty<B>`] values. It is used for storing function
/// parameter types and other ordered collections of types.
///
/// # Construction
///
/// Create a `TyList` using [`TyList::from_iter`], which takes a builder reference
/// and an iterator of [`Ty<B>`] values:
///
/// ```
/// use melbi_types::builders::BoxBuilder;
/// use melbi_types::{Ty, TyList, TyKind, Scalar};
///
/// let builder = BoxBuilder::new();
/// let int_ty = Ty::new(&builder, TyKind::Scalar(Scalar::Int));
/// let str_ty = Ty::new(&builder, TyKind::Scalar(Scalar::Str));
///
/// let params = TyList::from_iter(&builder, [int_ty, str_ty]);
/// assert_eq!(params.len(), 2);
/// ```
///
/// # Iteration and Access
///
/// `TyList` implements [`Deref`] to `[Ty<B>]`, providing slice-like access:
///
/// ```
/// use melbi_types::builders::BoxBuilder;
/// use melbi_types::{Ty, TyList, TyKind, Scalar};
///
/// let builder = BoxBuilder::new();
/// let int_ty = Ty::new(&builder, TyKind::Scalar(Scalar::Int));
/// let params = TyList::from_iter(&builder, [int_ty]);
///
/// // Use iter() for explicit iteration
/// for ty in params.iter() {
///     println!("{:?}", ty.kind());
/// }
///
/// // Or use slice methods via Deref
/// assert_eq!(params.len(), 1);
/// assert!(!params.is_empty());
/// ```
///
/// [`Deref`]: core::ops::Deref
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TyList<B: TyBuilder>(B::TyListHandle);

impl<B: TyBuilder> TyList<B> {
    /// Creates a new `TyList` from an iterator of types.
    ///
    /// The iterator must implement [`ExactSizeIterator`] to allow efficient
    /// pre-allocation of storage.
    pub fn from_iter(
        builder: &B,
        iter: impl IntoIterator<Item = Ty<B>, IntoIter: ExactSizeIterator>,
    ) -> Self {
        Self(builder.alloc_ty_list(iter))
    }

    /// Returns an iterator over the types in this list.
    pub fn iter(&self) -> core::slice::Iter<'_, Ty<B>> {
        self.0.iter()
    }
}

impl<B: TyBuilder> Deref for TyList<B> {
    type Target = [Ty<B>];

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<'a, B: TyBuilder> IntoIterator for &'a TyList<B> {
    type Item = &'a Ty<B>;
    type IntoIter = core::slice::Iter<'a, Ty<B>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

// === IdentList ===

/// An owned list of identifiers, backed by the builder's storage.
///
/// `IdentList<B>` is a thin wrapper around the builder's [`TyBuilder::IdentListHandle`],
/// representing a sequence of [`Ident<B>`] values. It is used for storing symbol
/// variants in [`TyKind::Symbol`].
///
/// # Construction
///
/// Create an `IdentList` using [`IdentList::from_iter`], which takes a builder
/// reference and an iterator of [`Ident<B>`] values:
///
/// ```
/// use melbi_types::builders::BoxBuilder;
/// use melbi_types::{Ident, IdentList};
///
/// let builder = BoxBuilder::new();
/// let ok = Ident::new(&builder, "ok");
/// let error = Ident::new(&builder, "error");
///
/// let symbols = IdentList::from_iter(&builder, [ok, error]);
/// assert_eq!(symbols.len(), 2);
/// ```
///
/// # Iteration and Access
///
/// `IdentList` implements [`Deref`] to `[Ident<B>]`, providing slice-like access:
///
/// ```
/// use melbi_types::builders::BoxBuilder;
/// use melbi_types::{Ident, IdentList};
///
/// let builder = BoxBuilder::new();
/// let pending = Ident::new(&builder, "pending");
/// let symbols = IdentList::from_iter(&builder, [pending]);
///
/// // Use iter() for explicit iteration
/// for ident in symbols.iter() {
///     println!("{}", ident.as_str());
/// }
///
/// // Or use slice methods via Deref
/// assert_eq!(symbols.len(), 1);
/// ```
///
/// [`TyKind::Symbol`]: super::TyKind::Symbol
/// [`Deref`]: core::ops::Deref
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IdentList<B: TyBuilder>(B::IdentListHandle);

impl<B: TyBuilder> IdentList<B> {
    /// Creates a new `IdentList` from an iterator of identifiers.
    ///
    /// The iterator must implement [`ExactSizeIterator`] to allow efficient
    /// pre-allocation of storage.
    pub fn from_iter(
        builder: &B,
        iter: impl IntoIterator<Item = Ident<B>, IntoIter: ExactSizeIterator>,
    ) -> Self {
        Self(builder.alloc_ident_list(iter))
    }

    /// Returns an iterator over the identifiers in this list.
    pub fn iter(&self) -> core::slice::Iter<'_, Ident<B>> {
        self.0.iter()
    }
}

impl<B: TyBuilder> Deref for IdentList<B> {
    type Target = [Ident<B>];

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<'a, B: TyBuilder> IntoIterator for &'a IdentList<B> {
    type Item = &'a Ident<B>;
    type IntoIter = core::slice::Iter<'a, Ident<B>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

// === FieldList ===

/// An list of fields backed by the builder's storage.
///
/// `FieldList<B>` is a thin wrapper around the builder's [`TyBuilder::FieldListHandle`],
/// containing a sequence of named fields as they are usually found in record-like
/// structures where each field is represented by a `(Ident<B>, Ty<B>)` pair.
///
/// # Construction
///
/// Create a `FieldList` using [`FieldList::from_iter`], which takes a builder
/// reference and an iterator of `(Ident<B>, Ty<B>)` tuples:
///
/// ```
/// use melbi_types::builders::BoxBuilder;
/// use melbi_types::{Ident, Ty, FieldList, TyKind, Scalar};
///
/// let builder = BoxBuilder::new();
/// let name = Ident::new(&builder, "count");
/// let ty = Ty::new(&builder, TyKind::Scalar(Scalar::Int));
///
/// let fields = FieldList::from_iter(&builder, [(name, ty)]);
/// assert_eq!(fields.len(), 1);
/// ```
///
/// # Iteration and Access
///
/// `FieldList` implements [`Deref`] to `[(Ident<B>, Ty<B>)]`, providing slice-like access:
///
/// ```
/// use melbi_types::builders::BoxBuilder;
/// use melbi_types::{Ident, Ty, FieldList, TyKind, Scalar};
///
/// let builder = BoxBuilder::new();
/// let fields = FieldList::from_iter(&builder, [
///     (Ident::new(&builder, "x"), Ty::new(&builder, TyKind::Scalar(Scalar::Int))),
/// ]);
///
/// for (name, ty) in fields.iter() {
///     println!("{}: {:?}", name.as_str(), ty.kind());
/// }
/// ```
///
/// [`TyKind::Record`]: super::TyKind::Record
/// [`Deref`]: core::ops::Deref
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

impl<B: TyBuilder> Deref for FieldList<B> {
    type Target = [(Ident<B>, Ty<B>)];

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<'a, B: TyBuilder> IntoIterator for &'a FieldList<B> {
    type Item = &'a (Ident<B>, Ty<B>);
    type IntoIter = core::slice::Iter<'a, (Ident<B>, Ty<B>)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
