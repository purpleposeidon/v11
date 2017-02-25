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
#[macro_use]
extern crate lazy_static;

use std::sync::*;
use std::collections::HashMap;
use rustc_serialize::{Decodable, Encodable};
use std::marker::PhantomData;
use std::any::Any;


pub mod constructor;
pub mod tables;
pub mod property;
pub mod intern;
pub mod joincore;
pub mod columns;


use intern::*;

/**
 * Trait that all storable types must implement.
 *
 * Types that implement this trait should also not implement `Drop`, although this is not yet
 * expressable, and is not presently required.
 * */
// FIXME: Ditch 'codable.
// FIXME: Ditch Debug
pub trait Storable : Sync + Copy + Sized + ::std::fmt::Debug + Decodable + Encodable /* + !Drop */ { }
impl<T> Storable for T where T: Sync + Copy + Sized + ::std::fmt::Debug + Decodable + Encodable /* + !Drop */ { }


pub type GuardedUniverse = Arc<RwLock<Universe>>;

/**
 * A context object whose reference should be passed around everywhere.
 * */
pub struct Universe {
    pub tables: HashMap<String, RwLock<GenericTable>>,
    pub properties: Vec<PBox>,
}
impl Universe {
    pub fn new() -> Universe {
        let mut ret = Self::with_no_properties();
        ret.add_properties();
        ret
    }

    pub fn with_no_properties() -> Universe {
        Universe {
            tables: HashMap::new(),
            properties: Vec::new(),
        }
    }

    pub fn guard(self) -> GuardedUniverse { Arc::new(RwLock::new(self)) }

    /**
     * Returns a string describing all the tables in the Universe. (But does not include their
     * contents.)
     * */
    pub fn info(&self) -> String {
        self.tables.iter().map(|(_, table)| {
            table.read().unwrap().info()
        }).collect::<Vec<String>>().join(" ")
    }

    pub fn table_names(&self) -> Vec<String> {
        self.tables.keys().map(String::clone).collect()
    }
}


use num_traits::int::PrimInt;

#[derive(Debug)]
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
impl<I: PrimInt + ::num_traits::ToPrimitive, T: TableName> Iterator for RowIdIterator<I, T> {
    type Item = GenericRowId<I, T>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.end { return None; }
        let ret = GenericRowId::new(self.i);
        self.i = self.i + I::one();
        Some(ret)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let u = (self.i - self.end).to_usize().unwrap();
        (u, Some(u))
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

fn desync_box<'a>(v: &'a PBox) -> &'a Any {
    use std::ops::Deref;
    v.deref()
}

fn desync_box_mut<'a>(v: &'a mut PBox) -> &'a mut Any {
    use std::ops::DerefMut;
    v.deref_mut()
}

