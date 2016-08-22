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
extern crate bit_vec;
extern crate num_traits;

use std::sync::*;
use std::collections::HashMap;
use rustc_serialize::{Decodable, Encodable};
use std::marker::PhantomData;
use std::any::Any;


pub mod macros;
pub mod property;
pub mod intern;
pub mod constructor;

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
pub trait Storable : Default + Sync + Copy + Sized + ::std::fmt::Debug + Decodable + Encodable + PartialOrd /* + !Drop */ { }

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
    (),
    // [char; 4], [char; 8], [char; 16], [char; 32], [char; 64],
}


/**
 * A context object that should be passed around everywhere.
 * */
pub struct Universe {
    tables: HashMap<String, RwLock<GenericTable>>,
    properties: Vec<(String, Box<Any>)>,
    // A vec would be better. Would require some global static stuff to assign id's to properties.
    // Kinda needs const_fn.
}
impl Universe {
    /** Create a new Universe. */
    pub fn new() -> Universe {
        Universe {
            tables: HashMap::new(),
            properties: Vec::new(),
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


use num_traits::int::PrimInt;

pub struct RowIdIterator<I: PrimInt, T> {
    i: I,
    end: I,
    rt: PhantomData<T>,
}
impl<I: PrimInt, T> RowIdIterator<I, T> {
    pub fn new(start: I, end: I) -> Self {
        RowIdIterator {
            i: start,
            end:  end,
            rt: PhantomData,
        }
    }
}
impl<I: PrimInt, T> Iterator for RowIdIterator<I, T> {
    type Item = GenericRowId<I, T>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.end { return None; }
        let ret = GenericRowId::new(self.i);
        self.i = self.i + I::one();
        Some(ret)
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

