//! Data structures for storing columnar elements.

use crate::Storable;
use crate::columns::TCol;

/// Stores data contiguously using the standard rust `Vec`.
/// This is ideal for tables that do not have rows added to them often.
#[derive(Debug)]
pub struct VecCol<E: Storable> {
    data: Vec<E>,
}
impl<E: Storable> TCol for VecCol<E> {
    type Element = E;

    fn new() -> Self { VecCol { data: Vec::new() } }

    fn len(&self) -> usize { self.data.len() }
    fn truncate(&mut self, len: usize) { self.data.truncate(len) }
    unsafe fn unchecked_index(&self, i: usize) -> &Self::Element { self.data.get_unchecked(i) }
    unsafe fn unchecked_index_mut(&mut self, i: usize) -> &mut Self::Element { self.data.get_unchecked_mut(i) }
    fn reserve(&mut self, n: usize) { self.data.reserve(n) }
    fn clear(&mut self) { self.data.clear() }
    fn push(&mut self, v: Self::Element) { self.data.push(v) }
    unsafe fn unchecked_swap(&mut self, a: usize, b: usize) {
        if cfg!(debug) {
            self.data.swap(a, b);
        } else {
            let pa: *mut E = self.unchecked_index_mut(a);
            let pb: *mut E = self.unchecked_index_mut(b);
            ::std::ptr::swap(pa, pb);
        }
    }

    type IntoIter = ::std::vec::IntoIter<Self::Element>;
    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

/// Temporary (hopefully) stub for avec.
/// Use this for tables that may be heavily extended at run-time.
// FIXME: Implement. Mostly just need some kind of page_size allocator.
pub type SegCol<E> = VecCol<E>;

extern crate bit_vec;
type BitVec = self::bit_vec::BitVec<u32>;
/*
fn bitvec_from_parts<E, R>(len: usize, mut data: Vec<u32>, err: E) -> Result<BitVec, R>
where E: FnOnce(usize, &'static str) -> R
{
    // to_bytes/from_bytes is super whack. Just reuse the Vec, dude!
    const S: usize = 32;
    let backing_len = data.len() * S;
    {
        // There are two length fields, unfortunately. Are they consistent?
        if len > backing_len {
            // Longer...
            return Err(err(data.len(), "`len` longer than `data.len`"));
        }
        if backing_len - len > S {
            // It's okay to be short,
            // but not so short that an element is in excess.
            return Err(err(data.len(), "`len` is excessively shorter than `data.len`"));
        }
    }
    let mut bits = BitVec::new();
    unsafe {
        ::std::mem::swap(bits.storage_mut(), &mut data);
        bits.set_len(backing_len);
        bits.truncate(len);
        // We don't just do `bits.set_len(backing_len)` because the bit-vec
        // docs talk about the importance of the excess bits being 0 for
        // "correctness". It's probably only relevant for BitSet operations,
        // which we don't use, but this is easy to do.
    }
    Ok(bits)
}
*/

/// Densely packed booleans.
#[derive(Debug)]
pub struct BoolCol {
    ref_val: bool,
    ref_idx: usize,
    data: BitVec,
}
impl Default for BoolCol {
    fn default() -> Self {
        BoolCol {
            ref_val: false,
            ref_idx: ::std::usize::MAX,
            data: BitVec::default(),
        }
    }
}
impl BoolCol {
    fn flush(&mut self) {
        if self.ref_idx >= self.len() { return }
        self.data.set(self.ref_idx, self.ref_val);
        self.ref_idx = ::std::usize::MAX;
    }
}
impl TCol for BoolCol {
    type Element = bool;

    fn new() -> BoolCol { Default::default() }

    fn len(&self) -> usize { self.data.len() }
    fn truncate(&mut self, len: usize) {
        self.flush();
        self.data.truncate(len)
    }
    unsafe fn unchecked_index(&self, i: usize) -> &Self::Element {
        // FIXME: Actually checked!
        if i == self.ref_idx {
            &self.ref_val
        } else if self.data[i] {
            &true
        } else {
            &false
        }
    }
    unsafe fn unchecked_index_mut(&mut self, i: usize) -> &mut Self::Element {
        self.flush();
        self.ref_idx = i;
        self.ref_val = self.data[i];
        &mut self.ref_val
    }
    fn reserve(&mut self, n: usize) { self.data.reserve(n) }
    fn push(&mut self, v: Self::Element) { self.data.push(v) }
    unsafe fn unchecked_swap_out(&mut self, i: usize, new: &mut Self::Element) {
        self.flush();
        let new_v = *new;
        *new = self.data[i];
        self.data.set(i, new_v);
    }
    unsafe fn unchecked_swap(&mut self, a: usize, b: usize) {
        self.flush();
        let av: bool = self.data[a];
        let bv: bool = self.data[b];
        self.data.set(a, bv);
        self.data.set(b, av);
    }

    type IntoIter = self::bit_vec::IntoIter;
    fn into_iter(mut self) -> Self::IntoIter {
        self.flush();
        self.data.into_iter()
    }
}

#[cfg(test)]
mod test {
    use super::{TCol, BoolCol};
    #[test]
    fn bool_col_unit() {
        let mut bc = BoolCol::new();
        let v = &[true, false, true];
        for i in v {
            bc.data.push(*i);
        }
        println!("");
        println!("Start:");
        for i in &bc.data {
            println!("{}", i);
        }
        println!("Cleared:");
        bc.clear();
        for i in &bc.data {
            println!("{}", i);
        }
        println!("Really Cleared:");
        bc.clear();
        for i in &bc.data {
            println!("{}", i);
        }
        assert!(bc.data.is_empty());
        println!("Append:");
        bc.data.extend(vec![true, false]);
        for i in &bc.data {
            println!("{}", i);
        }
        println!("{:?}", bc);
        unsafe {
            assert_eq!(bc.unchecked_index(0), &true);
            assert_eq!(bc.unchecked_index(1), &false);
        }
    }

    #[test]
    fn simple() {
        unsafe {
            let mut bc = BoolCol::new();
            println!("{:?}", bc.ref_idx);
            println!("{:?}", bc.ref_val);
            bc.data.push(true);
            println!("{:?}", bc.ref_idx);
            println!("{:?}", bc.ref_val);
            assert_eq!(bc.unchecked_index(0), &true);
            let mut bc = BoolCol::new();
            bc.data.push(false);
            assert_eq!(bc.unchecked_index(0), &false);
        }
    }
}
