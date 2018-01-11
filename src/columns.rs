//! Table indexing is strongly typed.
//! Each table has its own index type (`GenericRowId`),
//! which may be converted to a 'pre-checked' form (`CheckedRowId`).

use std::ops::{Index, IndexMut};
use std::marker::PhantomData;
use Storable;
use tables::{GetTableName, LockedTable, GenericRowId, CheckedRowId};

/// All storage types implement this trait.
// We might be able to live without this, tho it provides some 'module unsafety boundary' guarantees.
pub trait TCol {
    /// This is duck-typingly a `Vec`.
    type Data;
    type Element: Storable;

    fn new() -> Self where Self: Sized;
    fn data(&self) -> &Self::Data;
    fn data_mut(&mut self) -> &mut Self::Data;

    fn col_index(&self, i: usize) -> &Self::Element;
    fn col_index_mut(&mut self, i: usize) -> &mut Self::Element;
    unsafe fn col_index_unchecked(&self, i: usize) -> &Self::Element { self.col_index(i) }
    unsafe fn col_index_unchecked_mut(&mut self, i: usize) -> &mut Self::Element { self.col_index_mut(i) }
    // just here to properly constrain by the unsafety scope
    fn len(&self) -> usize;
}

/// It's not possible to do a blanket implementation of indexing on `TCol`s due to orphan rules,
/// so this is a wrapper.
// We could ditch the wrapper by `trait TCol: Index<...>`, but this is more pleasant to deal with.
pub struct Col<C: TCol, T: GetTableName> {
    inner: C,
    table: PhantomData<T>,
}
impl<C: TCol, T: GetTableName> Col<C, T> {
    pub fn new() -> Self {
        Self { inner: C::new(), table: PhantomData }
    }

    #[inline] pub fn data(&self) -> &C::Data { self.inner.data() }
    #[inline] pub fn data_mut(&mut self) -> &mut C::Data { self.inner.data_mut() }
}
impl<C: TCol, T: GetTableName> Index<GenericRowId<T>> for Col<C, T> {
    type Output = C::Element;
    fn index(&self, index: GenericRowId<T>) -> &Self::Output { self.inner.col_index(index.to_usize()) }
}
impl<C: TCol, T: GetTableName> IndexMut<GenericRowId<T>> for Col<C, T> {
    fn index_mut(&mut self, index: GenericRowId<T>) -> &mut Self::Output { self.inner.col_index_mut(index.to_usize()) }
}
impl<'a, C: TCol, T: LockedTable + 'a> Index<CheckedRowId<'a, T>> for Col<C, T::Row> {
    type Output = C::Element;
    fn index(&self, index: CheckedRowId<T>) -> &Self::Output { unsafe { self.inner.col_index_unchecked(index.to_usize()) } }
}
impl<'a, C: TCol, T: LockedTable + 'a> IndexMut<CheckedRowId<'a, T>> for Col<C, T::Row> {
    fn index_mut(&mut self, index: CheckedRowId<T>) -> &mut Self::Output { unsafe { self.inner.col_index_unchecked_mut(index.to_usize()) } }
}



/// A column that can be `Index`ed.
pub struct RefA<'a, T: 'a>(&'a T);
/// A column that can be `Index`ed and `IndexMut`ed.
pub struct MutA<'a, T: 'a>(&'a mut T);
/// A column that can be `Index`ed.
/// (And is secretly mutable by the implementation.)
pub struct EditA<'a, T: 'a>(&'a mut T);

impl<'a, T: 'a> RefA<'a, T> {
    pub fn new(t: &'a T) -> Self { RefA(t) }
    #[doc(hidden)] pub fn deref(&self) -> &T { self.0 }
}
impl<'a, T: 'a> MutA<'a, T> {
    pub fn new(t: &'a mut T) -> Self { MutA(t) }
    #[doc(hidden)] pub fn deref(&self) -> &T { self.0 }
    #[doc(hidden)] pub fn deref_mut(&mut self) -> &mut T { self.0 }
}
impl<'a, T: 'a> EditA<'a, T> {
    pub fn new(t: &'a mut T) -> Self { EditA(t) }
    #[doc(hidden)] pub fn deref(&self) -> &T { self.0 }
    #[doc(hidden)] pub fn deref_mut(&mut self) -> &mut T { self.0 }
}

// Type macros. Some day.
// FIXME: RefA's should use TCol directly.
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



