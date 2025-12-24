use bitflags::bitflags;

bitflags! {
    /// Flags indicating various properties of a type.
    ///
    /// These flags are computed once when a type is interned and cached
    /// for efficient queries. This avoids repeated recursive traversals.
    ///
    /// Starting with an empty set - flags will be added as we implement
    /// features that need them (inference vars, placeholders, bound vars, etc.)
    #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
    pub struct TyFlags: u16 {
        const HAS_TYPE_VARS = 1;
    }
}
