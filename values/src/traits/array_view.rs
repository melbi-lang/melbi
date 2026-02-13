pub trait ArrayView<E> {
    fn len(&self) -> usize;

    fn get(&self, index: usize) -> Option<E>;

    // TODO: Should we add this for efficiency?
    // fn get_unchecked(&self, index: usize) -> E;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
