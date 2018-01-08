//! Column storage types.

use std::ops::{Index, IndexMut};
use std::marker::PhantomData;
use Storable;
use tables::{GetTableName, LockedTable, GenericRowId, CheckedRowId};

/// All columns implement this trait.
pub trait TCol {
    /// This is a 'duck type'; it must be sufficiently `Vec`-like.
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

/// Stores data contiguously using the standard rust `Vec`.
/// This column type is ideal for tables that are 'static': written once, and then modified.
/// Indexing is slightly more efficient than a `SegCol`.
#[derive(Debug)]
#[derive(RustcEncodable, RustcDecodable)]
pub struct VecCol<E: Storable> {
    data: Vec<E>,
}
impl<E: Storable> TCol for VecCol<E> {
    type Data = Vec<E>;
    type Element = E;

    fn new() -> Self { VecCol { data: Vec::new() } }
    fn data(&self) -> &Self::Data { &self.data }
    fn data_mut(&mut self) -> &mut Self::Data { &mut self.data }

    fn col_index(&self, index: usize) -> &E { &self.data[index] }
    fn col_index_mut(&mut self, index: usize) -> &mut E { &mut self.data[index] }
    unsafe fn col_index_unchecked(&self, index: usize) -> &E { self.data.get_unchecked(index) }
    unsafe fn col_index_unchecked_mut(&mut self, index: usize) -> &mut E { self.data.get_unchecked_mut(index) }
    fn len(&self) -> usize { self.data.len() }
}

type BitVec = ::bit_vec::BitVec<u64 /* explicitly, not usize */>;

/// Densely packed booleans.
#[derive(Debug, Default)]
#[derive(RustcEncodable, RustcDecodable)]
pub struct BoolCol {
    data: BitVec,
}
impl TCol for BoolCol {
    type Data = BitVec;
    type Element = bool;
    fn new() -> BoolCol {
        Default::default()
    }

    fn data(&self) -> &Self::Data { &self.data }
    fn data_mut(&mut self) -> &mut Self::Data { &mut self.data }

    fn col_index(&self, index: usize) -> &bool { &self.data[index] }
    fn col_index_mut(&mut self, index: usize) -> &mut bool { &mut self.data[index] }
    fn len(&self) -> usize { self.data.len() }
}

/// Temporary (hopefully) stub for avec.
/// Use this for tables that may be heavily extended at run-time.
// FIXME: Implement.
pub type SegCol<E> = VecCol<E>;

#[cfg(test)]
mod test {
    #[test]
    fn bool_col_unit() {
        use super::TCol;
        let mut bc = super::BoolCol::new();
        let v = &[true, false, true];
        for i in v {
            bc.data_mut().push(*i);
        }
        println!("");
        println!("Start:");
        for i in bc.data().iter() {
            println!("{}", i);
        }
        println!("Cleared:");
        bc.data_mut().clear();
        for i in bc.data().iter() {
            println!("{}", i);
        }
        println!("Really Cleared:");
        bc.data_mut().clear();
        for i in bc.data().iter() {
            println!("{}", i);
        }
        println!("Append:");
        bc.data_mut().extend(vec![true, true]);
        for i in bc.data().iter() {
            println!("{}", i);
        }
        println!("{:?}", bc);
    }
}
