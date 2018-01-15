use columns::TCol;
use std::collections::BTreeMap;
use std::hash::Hash;

pub type BIndex<E> = BTreeMap<(E, usize), ()>;

pub trait IndexedCol: TCol {
    fn get_index(&self) -> &BIndex<Self::Element>;
}

pub struct BTreeIndex<T: TCol>
where T::Element: Hash + Ord + Copy
{
    inner: T,
    index: BIndex<T::Element>,
}
impl<T: TCol> IndexedCol for BTreeIndex<T>
where T::Element: Hash + Ord + Copy {
    fn get_index(&self) -> &BIndex<T::Element> { &self.index }
}
impl<T: TCol> TCol for BTreeIndex<T> where T::Element: Hash + Ord + Copy {
    type Element = T::Element;

    fn new() -> Self where Self: Sized {
        BTreeIndex {
            inner: T::new(),
            index: BIndex::new(),
        }
    }

    fn len(&self) -> usize { self.inner.len() }
    unsafe fn unchecked_index(&self, i: usize) -> &Self::Element { self.inner.unchecked_index(i) }
    //unsafe fn unchecked_index_mut(&mut self, i: usize) -> &mut Self::Element { self.inner.unchecked_index_mut(i) }
    unsafe fn unchecked_index_mut(&mut self, _i: usize) -> &mut Self::Element { panic!("tried to mutably reference indexed column"); }
    fn reserve(&mut self, n: usize) { self.inner.reserve(n); }
    fn clear(&mut self) {
        self.inner.clear();
        self.index.clear();
    }
    fn push(&mut self, v: Self::Element) {
        let i = self.inner.len();
        self.inner.push(v);
        self.index.insert((v, i), ());
    }

    unsafe fn unchecked_swap(&mut self, i: usize, new: &mut Self::Element) {
        let old = *self.unchecked_index(i);
        self.index.remove(&(old, i));
        self.index.insert((*new, i), ());
        self.inner.unchecked_swap(i, new);
    }

    unsafe fn deleted(&mut self, i: usize) {
        let old = *self.unchecked_index(i);
        self.index.remove(&(old, i));
        self.inner.deleted(i);
    }
}
