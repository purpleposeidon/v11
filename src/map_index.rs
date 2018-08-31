//! Columns can be made searchable using `#[index]`.
//! Such columns have their mutability restricted.
//! The index can be searched using `table.column.find(&element)`.

use std::collections::{BTreeMap, btree_map};
use std::hash::Hash;

use num_traits::NumCast;

use columns::TCol;
use tables::GetTableName;
use index::GenericRowId;

/// An iterator over the rows containing a searched-for element.
pub struct Indexes<'a, C: TCol + 'a, T: GetTableName + 'a> {
    range: btree_map::Range<'a, (C::Element, T::Idx), ()>,
}
impl<'a, C: TCol + 'a, T: GetTableName + 'a> Iterator for Indexes<'a, C, T> {
    type Item = GenericRowId<T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.range
            .next()
            .map(|v| (v.0).1)
            .map(GenericRowId::new)
    }
}

/// A `TCol` wrapper that does indexing. The element must be Ord.
pub struct BTreeIndex<C: TCol, T: GetTableName>
where C::Element: Hash + Ord + Copy
{
    inner: C,
    /// Unfortunately it duplicates the elements, but at least it is very easy to implement and does
    /// limited allocation.
    index: BTreeMap<(C::Element, T::Idx), ()>,
}
impl<C: TCol, T: GetTableName> BTreeIndex<C, T>
where C::Element: Hash + Ord + Copy
{
    /// Returns an iterator yielding the rows containing `key`.
    pub fn find(&self, key: C::Element) -> Indexes<C, T> {
        use num_traits::{Zero, Bounded};
        let zero = T::Idx::zero();
        let max = T::Idx::max_value();
        Indexes {
            // This excludes MAX. Unlikely! Can be fixed when `collections_range` lands.
            range: self.index.range((key, zero)..(key, max))
        }
    }
}
use serde::{Serialize, Serializer};
impl<C: TCol, T: GetTableName> Serialize for BTreeIndex<C, T>
where
    C::Element: Hash + Ord + Copy,
    C: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer
    {
        ::erased_serde::serialize(&self.inner, serializer)
    }
}
impl<C: TCol, T: GetTableName> TCol for BTreeIndex<C, T>
where C::Element: Hash + Ord + Copy
{
    type Element = C::Element;

    fn new() -> Self where Self: Sized {
        BTreeIndex {
            inner: C::new(),
            index: BTreeMap::new(),
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
        let native_i = NumCast::from(i).unwrap();
        self.index.insert((v, native_i), ());
    }

    unsafe fn unchecked_swap_out(&mut self, i: usize, new: &mut Self::Element) {
        let old = *self.unchecked_index(i);
        let native_i = NumCast::from(i).unwrap();
        self.index.remove(&(old, native_i));
        self.index.insert((*new, native_i), ());
        self.inner.unchecked_swap_out(i, new);
    }

    unsafe fn unchecked_swap(&mut self, a: usize, b: usize) {
        let old_a = *self.unchecked_index(a);
        let old_b = *self.unchecked_index(b);
        let native_a = NumCast::from(a).unwrap();
        let native_b = NumCast::from(b).unwrap();
        self.index.remove(&(old_a, native_a));
        self.index.remove(&(old_b, native_b));
        self.index.insert((old_a, native_b), ());
        self.index.insert((old_b, native_a), ());
        self.inner.unchecked_swap(a, b);
    }

    unsafe fn deleted(&mut self, i: usize) {
        let old = *self.unchecked_index(i);
        let native_i = NumCast::from(i).unwrap();
        self.index.remove(&(old, native_i));
        self.inner.deleted(i);
    }

    type IntoIter = C::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
