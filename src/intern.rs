//! This crate contains internal (but still public) items that are used by the `table!` and
//! `property!` macros.
//! User-code should not use this directly.
//! In particular, `use v11::intern::*` should be avoided as it causes import clashes.

use std::any::Any;
use std::sync::*;

use super::*;

impl Universe {
    pub fn add_table(&mut self, table: GenericTable) {
        if table.columns.is_empty() { panic!("Table has no columns"); }
        use std::collections::hash_map::Entry;
        match self.tables.entry(table.name.clone()) {
            Entry::Occupied(e) => {
                e.get().read().unwrap().assert_mergable(&table);
                // Maybe it's not unreasonable to be able to add columns.
            },
            Entry::Vacant(e) => { e.insert(RwLock::new(table)); },
        }
    }

    pub fn get_generic_table<'u, 's>(&'u self, name: &'s str) -> &'u RwLock<GenericTable> {
        match self.tables.get(name) {
            None => panic!("Table {} was not registered", name),
            Some(t) => t.clone(),
        }
    }
}

/// A table held by `Universe`. Its information is used to populate concrete tables.
pub struct GenericTable {
    pub name: String,
    pub is_sorted: bool,
    pub columns: Vec<GenericColumn>,
}
impl GenericTable {
    pub fn new(name: &str) -> GenericTable {
        check_name(name);
        GenericTable {
            name: name.to_string(),
            columns: Vec::new(),
            is_sorted: true,
        }
    }

    pub fn add_column(mut self, name: &str, type_name: String, inst: PBox) -> Self {
        // Why is the 'static necessary??? Does it refer to the vtable or something?
        check_name(name);
        for c in self.columns.iter() {
            if c.name == name {
                panic!("Duplicate column name {}", name);
            }
        }
        self.columns.push(GenericColumn {
            name: name.to_string(),
            data: inst,
            stored_type_name: type_name,
        });
        self
    }

    pub fn get_column<C: Any>(&self, name: &str, type_name: String) -> &C {
        let c = self.columns.iter().filter(|c| c.name == name).next().unwrap_or_else(|| {
            panic!("Table {} doesn't have a {} column.", self.name, name);
        });
        if c.stored_type_name != type_name { panic!("Column {}/{} has datatype {}, not {}", self.name, name, c.stored_type_name, type_name); }
        match ::desync_box(&c.data).downcast_ref() {
            Some(ret) => ret,
            None => {
                panic!("Column {}/{}: type conversion from {} to {} failed", self.name, name, c.stored_type_name, type_name);
            },
        }
    }

    pub fn get_column_mut<C: Any>(&mut self, name: &str, type_name: String) -> &mut C {
        let my_name = &self.name;
        let c = self.columns.iter_mut().filter(|c| c.name == name).next().unwrap_or_else(|| {
            panic!("Table {} doesn't have a {} column.", my_name, name);
        });
        if c.stored_type_name != type_name { panic!("Column {}/{} has datatype {}, not {}", self.name, name, c.stored_type_name, type_name); }
        match ::desync_box_mut(&mut c.data).downcast_mut() {
            Some(ret) => ret,
            None => {
                panic!("Column {}/{}: type conversion from {} to {} failed", self.name, name, c.stored_type_name, type_name);
            },
        }
    }

    fn assert_mergable(&self, other: &GenericTable) {
        if self.name != other.name {
            panic!("Mismatched table names: {:?} vs {:?}", self.name, other.name);
        }
        let crash = || {
            panic!("Tried to register table {} with incompatible columns!\nOld table: {}\nNew table: {}\n", other.name, self.info(), other.info());
        };
        if self.columns.len() != other.columns.len() {
            crash();
        }
        for (a, b) in self.columns.iter().zip(other.columns.iter()) {
            if a.name != b.name { crash(); }
            if a.stored_type_name != b.stored_type_name { crash(); }
        }
    }

    pub fn info(&self) -> String {
        let mut ret = format!("{}:", self.name);
        for col in self.columns.iter() {
            ret.push_str(&format!(" {}:[{}]", col.name, col.stored_type_name));
        }
        ret
    }
}

pub struct GenericColumn {
    name: String,
    stored_type_name: String,
    data: PBox,
}



fn check_name(name: &str) {
    match name.chars().next() {
        None => panic!("Empty name"),
        Some('_') => panic!("Reserved name {}", name),
        Some(c) if (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') => (),
        _ => panic!("Invalid name {}", name),
    }
    for c in name.chars() {
        if c == '_' { continue; }
        if c >= 'A' && c <= 'Z' { continue; }
        if c >= 'a' && c <= 'z' { continue; }
        if c >= '0' && c <= '9' { continue; }
        panic!("Invalid name {}", name);
    }
}


pub struct VoidIter<I>(I);
impl<I> Iterator for VoidIter<I> {
    type Item = I;
    fn next(&mut self) -> Option<I> { None }
}



// indexing



use std::marker::PhantomData;
use num_traits::PrimInt;
#[derive(Debug, Copy, Clone)]
pub struct GenericRowId<I: PrimInt, T> {
    i: I,
    t: PhantomData<T>,
}

impl<I: PrimInt, T> GenericRowId<I, T> {
    pub fn new(i: I) -> Self {
        GenericRowId {
            i: i,
            t: PhantomData,
        }
    }

    pub fn to_usize(&self) -> usize { self.i.to_usize().unwrap() }
    pub fn to_raw(&self) -> I { self.i }
    pub fn next(&self) -> Self {
        Self::new(self.i + I::one())
    }
}

use std::cmp::{Eq, PartialEq, PartialOrd, Ord};
impl<I: PrimInt, T> PartialEq for GenericRowId<I, T> {
    fn eq(&self, other: &GenericRowId<I, T>) -> bool {
        self.i == other.i
    }
}
impl<I: PrimInt, T> Eq for GenericRowId<I, T> {}
impl<I: PrimInt, T> PartialOrd for GenericRowId<I, T> {
    fn partial_cmp(&self, other: &GenericRowId<I, T>) -> Option<::std::cmp::Ordering> {
        self.i.partial_cmp(&other.i)
    }
}
impl<I: PrimInt, T> Ord for GenericRowId<I, T> {
    fn cmp(&self, other: &GenericRowId<I, T>) -> ::std::cmp::Ordering {
        self.i.cmp(&other.i)
    }
}

// Things get displeasingly manual due to the PhantomData.
use std::hash::{Hash, Hasher};
impl<I: PrimInt + Hash, T> Hash for GenericRowId<I, T> {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.i.hash(state);
    }
}

use rustc_serialize::{Encoder, Encodable, Decoder, Decodable};
impl<I: PrimInt + Encodable, T> Encodable for GenericRowId<I, T> {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        self.i.encode(s)
    }
}

impl<I: PrimInt + Decodable, T> Decodable for GenericRowId<I, T> {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        Ok(Self::new(try!(I::decode(d))))
    }
}



use std::ops::{Index, IndexMut, Range};


pub trait TCol<E: Storable> {
    fn new() -> Self;
    fn len(&self) -> usize;
    fn col_index(&self, i: usize) -> &E;
    fn col_index_mut(&mut self, i: usize) -> &mut E;
    fn clear(&mut self);
    fn push(&mut self, e: E);
    fn truncate(&mut self, l: usize);
    fn remove_slice(&mut self, range: Range<usize>);
    fn append(&mut self, other: &mut Vec<E>);
    fn reserve(&mut self, additional: usize);
}

pub struct ColWrapper<E: Storable, C: TCol<E>, R> {
    pub data: C,
    stored_type: PhantomData<E>,
    row_id_type: PhantomData<R>,
}
impl<E: Storable, C: TCol<E>, R> ColWrapper<E, C, R> {
    pub fn new() -> Self {
        ColWrapper {
            data: C::new(),
            stored_type: PhantomData,
            row_id_type: PhantomData,
        }
    }
}
impl<E: Storable, C: TCol<E>, R: PrimInt, T> Index<GenericRowId<R, T>> for ColWrapper<E, C, GenericRowId<R, T>> {
    type Output = E;
    fn index(&self, index: GenericRowId<R, T>) -> &E { self.data.col_index(index.to_usize()) }
}
impl<E: Storable, C: TCol<E>, R: PrimInt, T> IndexMut<GenericRowId<R, T>> for ColWrapper<E, C, GenericRowId<R, T>> {
    fn index_mut(&mut self, index: GenericRowId<R, T>) -> &mut E { self.data.col_index_mut(index.to_usize()) }
}

#[derive(Debug)]
pub struct VecCol<E: Storable> {
    data: Vec<E>,
}
impl<E: Storable> TCol<E> for VecCol<E> {
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


#[derive(Debug)]
pub struct BoolCol {
    data: ::bit_vec::BitVec,
    ref_id: Option<usize>,
    ref_val: bool,
}
impl BoolCol {
    fn flush(&mut self) {
        match self.ref_id {
            Some(i) => {
                self.data.set(i, self.ref_val);
                self.ref_id = None;
            },
            _ => (),
        }
    }
}
impl TCol<bool> for BoolCol {
    fn new() -> BoolCol {
        BoolCol {
            data: ::bit_vec::BitVec::new(),
            ref_id: None,
            ref_val: false,
        }
    }

    fn len(&self) -> usize { self.data.len() }

    fn col_index(&self, index: usize) -> &bool {
        match self.ref_id {
            Some(i) if i == index => &self.ref_val,
            _ => &self.data[index],
        }
    }

    fn col_index_mut(&mut self, index: usize) -> &mut bool {
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
pub type SegCol<E> = VecCol<E>;

/**
 * Hack to avoid issues with types, such as floats, that do not implement `Ord`.
 * In some circumstances you could move a column to the front; but this may not be desirable, and
 * is of course impossible in cases when all columns are not sortable.
 *
 * ```
 * # #[macro_use] extern crate v11;
 * extern crate rustc_serialize;
 * pub mod table_use {}
 * table! {
 *      [pub floating_table],
 *      unsorted: [(); VoidCol],
 *      float: [f32; SegCol<f32>],
 * }
 * # fn main() {}
 * ```
 * */
pub type VoidCol = VecCol<()>;

#[cfg(test)]
mod test {
    #[test]
    fn bool_col_unit() {
        use super::TCol;
        let mut bc = ::intern::BoolCol::new();
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

pub type PBox = Box<Any + Send + Sync>;
