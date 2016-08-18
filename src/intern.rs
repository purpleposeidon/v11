//! This crate contains internal (but still public) items that are used by the `table!` macro.
//! User-code should not use this directly.

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
            None => panic!("Table {} does not exist", name),
            Some(t) => t.clone(),
        }
    }
}

/// A table held by `Universe`. Its information is used to create populate concrete tables.
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

    pub fn add_column(mut self, name: &str, type_name: String, inst: Box<Any>) -> Self {
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
        if c.stored_type_name != type_name { panic!("Column {}:{} has datatype {}, not {}", self.name, name, c.stored_type_name, type_name); }
        c.data.downcast_ref().unwrap()
    }

    pub fn get_column_mut<C: Any>(&mut self, name: &str, type_name: String) -> &mut C {
        let my_name = &self.name;
        let c = self.columns.iter_mut().filter(|c| c.name == name).next().unwrap_or_else(|| {
            panic!("Table {} doesn't have a {} column.", my_name, name);
        });
        if c.stored_type_name != type_name { panic!("Column {}:{} has datatype {}, not {}", self.name, name, c.stored_type_name, type_name); }
        c.data.downcast_mut().unwrap()
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
    data: Box<Any>,
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

impl<D: Storable> VecCol<D> {
    pub fn new() -> Self { VecCol { data: Vec::new() } }
    pub fn len(&self) -> usize { self.data.len() }
    pub fn clear(&mut self) { self.data.clear() }
    pub fn push(&mut self, d: D) { self.data.push(d) }
    pub fn truncate(&mut self, l: usize) { self.data.truncate(l) }
    pub fn remove_slice(&mut self, range: ::std::ops::Range<usize>) {
        self.data.drain(range);
    }
    pub fn append(&mut self, other: &mut Vec<D>) { self.data.append(other) }
}

impl BoolCol {
    pub fn new() -> BoolCol {
        BoolCol {
            data: ::bit_vec::BitVec::new(),
            ref_id: None,
            ref_val: false,
        }
    }

    pub fn len(&self) -> usize { self.data.len() }
    pub fn clear(&mut self) {
        self.flush();
        // BitVec.clear: "Clears all bits in this vector." Leaving the size the same. bro. pls.
        // https://github.com/contain-rs/bit-vec/issues/16
        // Anyways.
        self.data.truncate(0);
    }
    pub fn push(&mut self, d: bool) {
        self.flush();
        self.data.push(d);
    }
    pub fn truncate(&mut self, l: usize) {
        self.flush();
        self.data.truncate(l);
    }
    pub fn remove_slice(&mut self, range: ::std::ops::Range<usize>) {
        self.flush();
        for i in range.clone() {
            let v = self.data[range.end + i];
            self.data.set(i, v);
        }
        self.truncate(range.end);
    }
    pub fn append(&mut self, other: &mut Vec<bool>) {
        self.flush();
        self.data.reserve(other.len());
        for v in other {
            self.data.push(*v);
        }
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
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
}
/*macro_rules! row_id_type {
    ($I:ty) => {
        impl<T> GenericRowId<$I, T> {
            pub fn new(i: $I) -> Self {
                GenericRowId {
                    i: i,
                    t: PhantomData,
                }
            }

            pub fn to(&self) -> usize { self.i as usize }
        }
    }
}
row_id_type!{u8}
row_id_type!{u16}
row_id_type!{u32}
row_id_type!{u64}
row_id_type!{usize}*/

/* Still need to get a JOIN solution! */

