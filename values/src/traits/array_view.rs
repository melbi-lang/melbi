pub trait ArrayView<E> {
    fn len(&self) -> usize;

    fn get(&self, index: usize) -> Option<E>;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn iter(&self) -> impl Iterator<Item = E> + '_
    where
        Self: Sized,
    {
        (0..self.len()).map(move |i| self.get(i).unwrap())
    }
}
