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

/**
 * A column that's been locked for reading or writing and can be indexed using `OpaqueIndex`.
 * */
pub trait ConcreteCol<D: Storable> {
    fn get(&self, index: usize) -> &D;
    fn get_mut(&mut self, index: usize) -> &mut D;
}
impl<D: Storable> ::std::ops::Index<OpaqueIndex> for ConcreteCol<D> {
    type Output = D;
    fn index(&self, index: OpaqueIndex) -> &D { self.get(index.0) }
}
impl<D: Storable> ::std::ops::IndexMut<OpaqueIndex> for ConcreteCol<D> {
    fn index_mut(&mut self, index: OpaqueIndex) -> &mut D { self.get_mut(index.0) }
}

impl<D: Storable> ConcreteCol<D> for Vec<D> {
    fn get<'a>(&'a self, index: usize) -> &'a D {
        use std::ops::Index;
        Vec::<D>::index(self, index)
    }
    fn get_mut<'a>(&'a mut self, index: usize) -> &'a mut D {
        use std::ops::IndexMut;
        Vec::<D>::index_mut(self, index)
    }
}





/** An index into a table. It is a bad idea to be dependent on the contents of this value, as
* tables may be sorted asynchronously/you would have to keep things updated, etc. Consider using an
* explicit index column.
*
* TODO: Add a lifetime & table-type parameter to guarantee more safety.
* */
pub struct OpaqueIndex(usize);
impl OpaqueIndex {
    /** Get the underlying index value. */
    pub unsafe fn extricate(&self) -> usize {
        self.0
    }

    /** Create a phoney OpaqueIndex. */
    pub unsafe fn fabricate(i: usize) -> OpaqueIndex {
        OpaqueIndex(i)
    }
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



