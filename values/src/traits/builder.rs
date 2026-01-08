use core::fmt::Debug;

use melbi_types::{Ty, TyBuilder};

use crate::typed::ArrayView;

pub trait ValueBuilder: Sized + Clone {
    /// The type builder used for type representation.
    type TB: TyBuilder;
    /// The raw representation of the value.
    /// Example: `RawValue` (untagged union), or an enum.
    type Raw;
    /// The handle to the value.
    /// Example: `Value<Self>`, `Rc<Value<Self>>`
    type ValueHandle: AsRef<Value<Self>> + Clone + Debug;

    type Array: ArrayView<Value<Self>>;

    fn alloc(&self, value: Value<Self>) -> Self::ValueHandle;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Val<VB: ValueBuilder>(VB::ValueHandle);

impl<VB: ValueBuilder> Val<VB> {
    pub fn new(builder: &VB, value: Value<VB>) -> Self {
        Val(builder.alloc(value))
    }

    pub fn value(&self) -> &Value<VB> {
        self.0.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value<VB: ValueBuilder> {
    raw: VB::Raw,
    ty: Ty<VB::TB>,
}

impl<VB: ValueBuilder> Value<VB> {
    pub fn new(ty: Ty<VB::TB>, raw: VB::Raw) -> Self {
        Self { raw, ty }
    }

    pub fn raw(&self) -> &VB::Raw {
        &self.raw
    }

    pub fn ty(&self) -> &Ty<VB::TB> {
        &self.ty
    }

    pub fn alloc(self, builder: &VB) -> Val<VB> {
        Val::new(builder, self)
    }
}
