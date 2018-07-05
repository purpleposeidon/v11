//! Data structures for storing columnar elements.

use Storable;
use columns::TCol;

/// Stores data contiguously using the standard rust `Vec`.
/// This is ideal for tables that do not have rows added to them often.
#[derive(Debug)]
#[derive(RustcEncodable, RustcDecodable)]
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
}

/// Temporary (hopefully) stub for avec.
/// Use this for tables that may be heavily extended at run-time.
// FIXME: Implement. Mostly just need some kind of page_size allocator.
pub type SegCol<E> = VecCol<E>;

type BitVec = ::bit_vec::BitVec<u64>;

/// Densely packed booleans.
#[derive(Debug, Default)]
#[derive(RustcEncodable, RustcDecodable)]
pub struct BoolCol {
    data: BitVec,
}
impl TCol for BoolCol {
    type Element = bool;

    fn new() -> BoolCol { Default::default() }

    fn len(&self) -> usize { self.data.len() }
    fn truncate(&mut self, len: usize) { self.data.truncate(len) }
    unsafe fn unchecked_index(&self, i: usize) -> &Self::Element { &self.data[i] } // FIXME: be unchecked
    unsafe fn unchecked_index_mut(&mut self, i: usize) -> &mut Self::Element { &mut self.data[i] }
    fn reserve(&mut self, n: usize) { self.data.reserve(n) }
    fn push(&mut self, v: Self::Element) { self.data.push(v) }
    unsafe fn unchecked_swap_out(&mut self, i: usize, new: &mut Self::Element) {
        let new_v = *new;
        *new = self.data[i];
        self.data.set(i, new_v);
    }
    unsafe fn unchecked_swap(&mut self, a: usize, b: usize) {
        let av = self.data[a];
        let bv = self.data[b];
        self.data[a] = bv;
        self.data[a] = av;
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn bool_col_unit() {
        use super::TCol;
        let mut bc = super::BoolCol::new();
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
}
