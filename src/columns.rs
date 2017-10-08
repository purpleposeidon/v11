//! Column storage types.

use std::ops::{Index, IndexMut, Range};
use std::marker::PhantomData;
use num_traits::PrimInt;
use Storable;
use tables::{GetTableName, GenericRowId};

/// All columns must implement this trait.
/// It exposes an interface similar to `Vec`.
pub trait TCol {
    type Element: Storable;
    fn new() -> Self;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool { self.len() == 0 }
    fn col_index(&self, i: usize) -> &Self::Element;
    fn col_index_mut(&mut self, i: usize) -> &mut Self::Element;
    fn clear(&mut self);
    fn push(&mut self, e: Self::Element);
    fn truncate(&mut self, l: usize);
    fn remove_slice(&mut self, range: Range<usize>);
    fn append(&mut self, other: &mut Vec<Self::Element>);
    fn reserve(&mut self, additional: usize);
}

/// Column wrapper, used by tables. Internal.
#[derive(RustcEncodable, RustcDecodable)]
pub struct ColWrapper<C: TCol, R> {
    pub data: C,
    stored_type: PhantomData<C::Element>,
    row_id_type: PhantomData<R>,
}
impl<C: TCol, R> ColWrapper<C, R> {
    pub fn new() -> Self {
        ColWrapper {
            data: C::new(),
            stored_type: PhantomData,
            row_id_type: PhantomData,
        }
    }
}
impl<C: TCol, R> TCol for ColWrapper<C, R> {
    type Element = C::Element;
    fn new() -> Self { ColWrapper::new() }
    fn len(&self) -> usize { self.data.len() }
    fn col_index(&self, i: usize) -> &Self::Element { self.data.col_index(i) }
    fn col_index_mut(&mut self, i: usize) -> &mut Self::Element { self.data.col_index_mut(i) }
    fn clear(&mut self) { self.data.clear() }
    fn push(&mut self, e: Self::Element) { self.data.push(e) }
    fn truncate(&mut self, l: usize) { self.data.truncate(l) }
    fn remove_slice(&mut self, range: Range<usize>) { self.data.remove_slice(range) }
    fn append(&mut self, other: &mut Vec<Self::Element>) { self.data.append(other) }
    fn reserve(&mut self, additional: usize) { self.data.reserve(additional) }
}
impl<C: TCol, R: PrimInt, T: GetTableName> Index<GenericRowId<R, T>> for ColWrapper<C, GenericRowId<R, T>> {
    type Output = C::Element;
    fn index(&self, index: GenericRowId<R, T>) -> &Self::Output { self.data.col_index(index.to_usize()) }
}
impl<C: TCol, R: PrimInt, T: GetTableName> IndexMut<GenericRowId<R, T>> for ColWrapper<C, GenericRowId<R, T>> {
    fn index_mut(&mut self, index: GenericRowId<R, T>) -> &mut Self::Output { self.data.col_index_mut(index.to_usize()) }
}

/// Stores data contiguously using the standard rust `Vec`.
/// This column type is ideal for tables that are 'static': written once, and then modified.
/// Indexing is slightly more efficient than a `SegCol`.
#[derive(Debug)]
#[derive(RustcEncodable, RustcDecodable)]
pub struct VecCol<E: Storable> {
    pub data: Vec<E>,
}
impl<E: Storable> TCol for VecCol<E> {
    type Element = E;
    fn new() -> Self { VecCol { data: Vec::new() } }
    fn len(&self) -> usize { self.data.len() }
    fn col_index(&self, index: usize) -> &E { &self.data[index] }
    fn col_index_mut(&mut self, index: usize) -> &mut E { &mut self.data[index] }
    fn clear(&mut self) { self.data.clear() }
    fn push(&mut self, d: E) { self.data.push(d) }
    fn truncate(&mut self, l: usize) { self.data.truncate(l) }
    fn remove_slice(&mut self, range: Range<usize>) { self.data.drain(range); } // FIXME: Might be wasteful; could be a better way.
    fn append(&mut self, other: &mut Vec<E>) { self.data.append(other) }
    fn reserve(&mut self, additional: usize) { self.data.reserve(additional) }
}


/// Densely packed booleans.
#[derive(Debug, Default)]
#[derive(RustcEncodable, RustcDecodable)]
pub struct BoolCol {
    data: ::bit_vec::BitVec<u64 /* explicit! */>,
    ref_id: Option<usize>,
    ref_val: bool,
}
impl BoolCol {
    fn flush(&mut self) {
        if let Some(i) = self.ref_id {
            self.data.set(i, self.ref_val);
            self.ref_id = None;
        }
    }
}
impl TCol for BoolCol {
    type Element = bool;
    fn new() -> BoolCol {
        Default::default()
    }

    fn len(&self) -> usize { self.data.len() }

    fn col_index(&self, index: usize) -> &bool {
        match self.ref_id {
            Some(i) if i == index => &self.ref_val,
            _ => &self.data[index],
        }
    }

    fn col_index_mut(&mut self, index: usize) -> &mut bool {
        // Return a reference to a buffer.
        // What happens if we get the &mut, change, & then drop? Well, all the other functions call
        // either flush() [if mut] or check ref_id [if ref].
        self.flush();
        self.ref_id = Some(index);
        self.ref_val = self.data[index];
        &mut self.ref_val
    }

    fn clear(&mut self) {
        self.flush();
        // BitVec.clear: "Clears all bits in this vector." Leaving the size the same. bro. pls.
        // https://github.com/contain-rs/bit-vec/issues/16
        // Anyways.
        self.data.truncate(0);
    }

    fn push(&mut self, d: bool) {
        self.flush();
        self.data.push(d);
    }

    fn truncate(&mut self, l: usize) {
        self.flush();
        self.data.truncate(l);
    }

    fn remove_slice(&mut self, range: ::std::ops::Range<usize>) {
        self.flush();
        for i in range.clone() {
            let v = self.data[range.end + i];
            self.data.set(i, v);
        }
        self.data.truncate(range.end);
    }

    fn append(&mut self, other: &mut Vec<bool>) {
        self.flush();
        self.data.reserve(other.len());
        for v in other {
            self.data.push(*v);
        }
    }

    fn reserve(&mut self, additional: usize) { self.data.reserve(additional) }
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
            bc.push(*i);
        }
        println!("");
        println!("Start:");
        for i in bc.data.iter() {
            println!("{}", i);
        }
        println!("Cleared:");
        bc.clear();
        for i in bc.data.iter() {
            println!("{}", i);
        }
        println!("Really Cleared:");
        bc.data.clear();
        for i in bc.data.iter() {
            println!("{}", i);
        }
        println!("Append:");
        bc.append(&mut vec![true, true]);
        for i in bc.data.iter() {
            println!("{}", i);
        }
        println!("{:?}", bc);
    }
}
