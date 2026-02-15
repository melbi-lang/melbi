use melbi_types::Ty;

use crate::traits::{ArrayView, ValueBuilder};

use super::Value;

/// A view into an array value.
///
/// Stores the element type so that when elements are accessed,
/// they can be returned as properly typed [`Value`]s.
#[derive(Debug, Clone)]
pub struct Array<B: ValueBuilder> {
    element_ty: Ty<B::TB>,
    handle: B::ArrayHandle,
}

impl<B: ValueBuilder> Array<B> {
    /// Create a new array view.
    ///
    /// `element_ty` is the type of elements in this array.
    pub(crate) fn new(element_ty: Ty<B::TB>, handle: B::ArrayHandle) -> Self {
        Self { element_ty, handle }
    }
}

impl<B: ValueBuilder> ArrayView<Value<B>> for Array<B> {
    fn len(&self) -> usize {
        self.handle.as_ref().len()
    }

    fn get(&self, index: usize) -> Option<Value<B>> {
        let elem_handle = self.handle.as_ref().get(index)?;
        // element_ty is cloned per access: free for arena (Copy) but involves
        // Rc ref-count bumps for box builders. Intentional trade-off to store
        // the type once and re-attach per element.
        Some(Value::new(self.element_ty.clone(), elem_handle.clone()))
    }

    fn iter(&self) -> impl Iterator<Item = Value<B>> + '_ {
        self.handle
            .as_ref()
            .iter()
            .map(|h| Value::new(self.element_ty.clone(), h.clone()))
    }
}
