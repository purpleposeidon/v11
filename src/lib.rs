//! A column-based in-memory database for [Data-Oriented
//! Programming](http://www.dataorienteddesign.com/dodmain/).
//! The tables and columns are dynamically described, but user-code interacts with them using
//! static dispatch. This requires the use of a macro to create a statically dispatched view of the
//! table, which does cause duplication of code & schema. However, this allows a large program to
//! dynamically load/hotswap libraries, and the crate for each library can have minimal
//! dependencies, and so can compile faster.
// I'm almost certain that's how that works.
//! 

extern crate rustc_serialize;
extern crate itertools;
extern crate joinkit;

use std::sync::*;
use std::collections::HashMap;
use rustc_serialize::{Decodable, Encodable};


pub mod macros;

pub mod intern;

#[cfg(test)]
mod test;

pub use intern::*;

/**
 * Trait that all storable types must implement.
 *
 * Types that implement this trait should also not implement `Drop`, although this is not yet
 * expressable, and is not presently required.
 * */
// We really do want to be able to store floats, which means that we can't use proper Eq or
// PartialEq...
pub trait Storable : Default + Sync + Copy + Sized + Decodable + Encodable + PartialOrd /* + !Drop */ { }

macro_rules! storables_table {
    ($($T:ty),*,) => {
        $(impl Storable for $T {})*
    }
}
storables_table! {
    i8, i16, i32, i64,
    u8, u16, u32, u64,
    isize, usize,
    f32, f64,
    bool, char,
    // [char; 4], [char; 8], [char; 16], [char; 32], [char; 64],
}


/**
 * A context object that should be passed around everywhere.
 * */
pub struct Universe {
    tables: HashMap<String, RwLock<GenericTable>>,
    /* TODO: Properties. */
}
impl Universe {
    /** Create a new Universe. */
    pub fn new() -> Universe {
        Universe {
            tables: HashMap::new(),
        }
    }

    /**
     * Returns a string describing all the tables in the Universe. (But does not include their
     * contents.)
     * */
    pub fn info(&self) -> String {
        self.tables.iter().map(|(_, table)| {
            table.read().unwrap().info()
        }).collect::<Vec<String>>().join(" ")
    }
}


use std::marker::PhantomData;

/** An index into a table. It is a bad idea to be dependent on the contents of this value, as
* tables may be sorted asynchronously/you would have to keep things updated, etc. Consider using an
* explicit index column.
*
* TODO: Add a lifetime to guarantee more safety.
* */
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpaqueIndex<T> {
    i: usize,
    t: PhantomData<T>
}
impl<T> OpaqueIndex<T> {
    /** Get the underlying index value. */
    pub unsafe fn extricate(&self) -> usize {
        self.i
    }

    /** Create a phoney OpaqueIndex. */
    pub unsafe fn fabricate(i: usize) -> OpaqueIndex<T> {
        OpaqueIndex::new(i)
    }

    fn new(i: usize) -> OpaqueIndex<T> {
        OpaqueIndex {
            i: i,
            t: PhantomData,
        }
    }
}

pub struct RowIndexIterator<Row> {
    i: usize,
    end: usize,
    rt: PhantomData<Row>,
}
impl<Row> Iterator for RowIndexIterator<Row> {
    type Item = OpaqueIndex<Row>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.end { return None; }
        let ret = OpaqueIndex {
            i: self.i,
            t: PhantomData,
        };
        self.i += 1;
        Some(ret)
    }
}




/**
 * A column that's been locked for reading or writing and can be indexed using `OpaqueIndex`.
 * */
pub struct Col<D: Storable> {
    data: Vec<D>,
}
impl<D: Storable, T> ::std::ops::Index<OpaqueIndex<T>> for Col<D> {
    type Output = D;
    fn index(&self, index: OpaqueIndex<T>) -> &D { &self.data[index.i] }
}
impl<D: Storable, T> ::std::ops::IndexMut<OpaqueIndex<T>> for Col<D> {
    fn index_mut(&mut self, index: OpaqueIndex<T>) -> &mut D { &mut self.data[index.i] }
}






/**
 * Return value for advanced iterators. Used for `$table::Write.visit()`
 * */
pub enum Action<I, IT: Iterator<Item=I>> {
    /// Nothing more needs to be iterated over.
    Break,
    /// Calls the closure with the next row, unless there is no more data.
    Continue,
    /// Remove the row that was just passed in.
    Remove,
    /// Add an arbitrary number of rows, after the provided row, using a move iterator.
    /// The rows inserted in this manner will not be walked by the closure.
    /// If you want to do a Remove and Add at the same time, move the first item in the iterator
    /// into the passed in row.
    Add(IT),
}


/* Still need to get a JOIN solution! */



