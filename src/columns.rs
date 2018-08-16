//! Table indexing is strongly typed.
//! Each table has its own index type (`GenericRowId`),
//! which may be converted to an 'already checked' form (`CheckedRowId`).
// FIXME: RowIdPreCheck, RowIdPostCheck?

use std::ops::{Index, IndexMut};
use std::marker::PhantomData;
use Storable;
use tables::{GetTableName, LockedTable, GenericRowId, CheckedRowId};

/// `Any` version of a column
pub trait AnyCol: ::mopa::Any + Send + Sync {}
mopafy!(AnyCol);

impl<T> AnyCol for T
where
    T: ::mopa::Any + Send + Sync
{}

/// All column storage types use this trait to expose a `Vec`-like interface.
/// Some of the methods are used to keep `IndexedCol`s in sync.
pub trait TCol: AnyCol {
    type Element: Storable;

    fn new() -> Self where Self: Sized;

    fn len(&self) -> usize;
    fn truncate(&mut self, len: usize);
    unsafe fn unchecked_index(&self, i: usize) -> &Self::Element;
    unsafe fn unchecked_index_mut(&mut self, i: usize) -> &mut Self::Element;
    fn reserve(&mut self, n: usize);
    fn clear(&mut self) { self.truncate(0) }
    fn push(&mut self, v: Self::Element);

    unsafe fn unchecked_swap_out(&mut self, i: usize, new: &mut Self::Element) { ::std::mem::swap(self.unchecked_index_mut(i), new) }
    unsafe fn unchecked_swap(&mut self, a: usize, b: usize);
    /// Callback for when an element is deleted.
    unsafe fn deleted(&mut self, _i: usize) {}

    fn checked_index(&self, i: usize) -> &Self::Element {
        if i >= self.len() {
            panic!("Index out of range: {}; length {}", i, self.len());
        }
        unsafe { self.unchecked_index(i) }
    }
}


/// It's not possible to do a blanket implementation of indexing on `TCol`s due to orphan rules,
/// so this is a wrapper.
// We could ditch the wrapper by `trait TCol: Index<...>`, but this is more pleasant to deal with.
pub struct Col<C: TCol, T: GetTableName> {
    inner: C,
    table: PhantomData<T>,
}
impl<C: TCol, T: GetTableName> Col<C, T> {
    #[doc(hidden)]
    pub fn new() -> Self {
        Self { inner: C::new(), table: PhantomData }
    }

    fn check(&self, i: usize) -> usize {
        if i >= self.inner.len() {
            panic!("Index out of range: Size is {}, but index is {}", self.inner.len(), i);
        }
        i
    }

    #[doc(hidden)] #[inline(always)] pub fn inner(&self) -> &C { &self.inner }
    #[doc(hidden)] #[inline(always)] pub fn inner_mut(&mut self) -> &mut C { &mut self.inner }
}
impl<C: TCol, T: GetTableName> Index<GenericRowId<T>> for Col<C, T> {
    type Output = C::Element;
    fn index(&self, i: GenericRowId<T>) -> &Self::Output {
        unsafe {
            let i = self.check(i.to_usize());
            self.inner.unchecked_index(i)
        }
    }
}
impl<C: TCol, T: GetTableName> IndexMut<GenericRowId<T>> for Col<C, T> {
    fn index_mut(&mut self, i: GenericRowId<T>) -> &mut Self::Output {
        unsafe {
            let i = self.check(i.to_usize());
            self.inner.unchecked_index_mut(i)
        }
    }
}
impl<'a, C: TCol, T: LockedTable + 'a> Index<CheckedRowId<'a, T>> for Col<C, T::Row> {
    type Output = C::Element;
    fn index(&self, index: CheckedRowId<T>) -> &Self::Output {
        unsafe {
            self.inner.unchecked_index(index.to_usize())
        }
    }
}
impl<'a, C: TCol, T: LockedTable + 'a> IndexMut<CheckedRowId<'a, T>> for Col<C, T::Row> {
    fn index_mut(&mut self, index: CheckedRowId<T>) -> &mut Self::Output {
        unsafe {
            self.inner.unchecked_index_mut(index.to_usize())
        }
    }
}


/// A `RefA` is a column that can be `Index`ed.
///
/// `RefA`, `MutA`, and `EditA` are wrappers that expose one interface to the world, but have a
/// hidden interface for `table!` to use.
pub struct RefA<'a, T: 'a>(&'a T);
/// A `MutA` is a column that can be `Index`ed and `IndexMut`ed.
///
/// `RefA`, `MutA`, and `EditA` are wrappers that expose one interface to the world, but have a
/// hidden interface for `table!` to use.
pub struct MutA<'a, T: 'a>(&'a mut T);
/// A `EditA` is a column that can be `Index`ed.
/// (And is secretly mutable by v11.)
/// Elements of such a column can be mutated, but no structural modifications are allowed.
///
/// `RefA`, `MutA`, and `EditA` are wrappers that expose one interface to the world, but have a
/// hidden interface for `table!` to use.
pub struct EditA<'a, T: 'a>(&'a mut T);

#[doc(hidden)]
impl<'a, T: 'a> RefA<'a, T> {
    pub fn new(t: &'a T) -> Self { RefA(t) }
    pub fn deref(&self) -> &T { self.0 }
}
#[doc(hidden)]
impl<'a, T: 'a> MutA<'a, T> {
    pub fn new(t: &'a mut T) -> Self { MutA(t) }
    pub fn deref(&self) -> &T { self.0 }
    pub fn deref_mut(&mut self) -> &mut T { self.0 }
}
#[doc(hidden)]
impl<'a, T: 'a> EditA<'a, T> {
    pub fn new(t: &'a mut T) -> Self { EditA(t) }
    pub fn deref(&self) -> &T { self.0 }
    pub fn deref_mut(&mut self) -> &mut T { self.0 }
}

// Forward indexing operations to `Col`.
impl<'a, I, T: Index<I> + 'a> Index<I> for RefA<'a, T> {
    type Output = T::Output;
    fn index(&self, i: I) -> &T::Output { &self.0[i] }
}
impl<'a, I, T: Index<I> + 'a> Index<I> for MutA<'a, T> {
    type Output = T::Output;
    fn index(&self, i: I) -> &T::Output { &self.0[i] }
}
impl<'a, I, T: Index<I> + 'a> Index<I> for EditA<'a, T> {
    type Output = T::Output;
    fn index(&self, i: I) -> &T::Output { &self.0[i] }
}

impl<'a, I, T: IndexMut<I> + 'a> IndexMut<I> for MutA<'a, T> {
    fn index_mut(&mut self, i: I) -> &mut T::Output { &mut self.0[i] }
}

mod searching {
    use super::*;
    use std::hash::Hash;
    use map_index::{Indexes, BTreeIndex};

    macro_rules! search_on {
        ($ty:ident) => {
            impl<'a, C, T> $ty<'a, Col<BTreeIndex<C, T>, T>>
            where
                C: TCol + 'a,
                T: GetTableName,
                C::Element: Hash + Ord + Copy,
            {
                pub fn find<'b>(&'a self, e: C::Element) -> Indexes<'b, C, T>
                where 'a: 'b
                {
                    self.deref().inner().find(e)
                }
            }
        };
    }

    search_on!(RefA);
    search_on!(MutA);
    search_on!(EditA);
}
