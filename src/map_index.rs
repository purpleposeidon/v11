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
    fn truncate(&mut self, new_len: usize) {
        // lame; probably no better way
        unsafe {
            for i in new_len..self.len() {
                self.deleted(i);
            }
        }
        self.inner.truncate(new_len);
    }
    unsafe fn unchecked_index(&self, i: usize) -> &Self::Element { self.inner.unchecked_index(i) }
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

    unsafe fn unchecked_swap_out(&mut self, i: usize, new: &mut Self::Element) {
        let old = *self.unchecked_index(i);
        self.index.remove(&(old, i));
        self.index.insert((*new, i), ());
        self.inner.unchecked_swap_out(i, new);
    }

    unsafe fn unchecked_swap(&mut self, a: usize, b: usize) {
        let old_a = *self.unchecked_index(a);
        let old_b = *self.unchecked_index(b);
        self.index.remove(&(old_a, a));
        self.index.remove(&(old_b, b));
        self.index.insert((old_a, b), ());
        self.index.insert((old_b, a), ());
        self.inner.unchecked_swap(a, b);
    }

    unsafe fn deleted(&mut self, i: usize) {
        let old = *self.unchecked_index(i);
        self.index.remove(&(old, i));
        self.inner.deleted(i);
    }
}
