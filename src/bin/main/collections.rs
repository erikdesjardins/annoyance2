use heapless::Vec;

pub trait HasElem {
    type Elem;
}

pub trait ReplaceWithMapped<Source: HasElem>: HasElem {
    fn replace_with_mapped(&mut self, other: &Source, f: impl FnMut(&Source::Elem) -> Self::Elem);
}

impl<T, const N: usize> HasElem for Vec<T, N> {
    type Elem = T;
}

impl<A, B, const N: usize> ReplaceWithMapped<Vec<A, N>> for Vec<B, N> {
    fn replace_with_mapped(&mut self, other: &Vec<A, N>, f: impl FnMut(&A) -> B) {
        // guaranteed not to overflow since both sides have the same capacity (N)
        self.clear();
        self.extend(other.into_iter().map(f));
    }
}
