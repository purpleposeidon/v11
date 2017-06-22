//! A column-based in-memory database for [Data-Oriented
//! Programming](http://www.dataorienteddesign.com/dodmain/).
//! The tables and columns are dynamically described, but user-code interacts with them using
//! static dispatch. This requires the use of a macro to create a statically dispatched view of the
//! table, which does cause duplication of code & schema. However, this allows a large program to
//! dynamically load/hotswap libraries, and the crate for each library can have minimal
//! dependencies, and so can compile faster.
// I'm almost certain that's how that works.
//! 

#[allow(unused_imports)]
#[macro_use]
// We don't actually use macros or the derive, but this silences up a warning.
extern crate v11_macros;
#[macro_use]
extern crate procedural_masquerade;
extern crate rustc_serialize;
extern crate itertools;
extern crate joinkit;
extern crate bit_vec;
extern crate num_traits;
#[macro_use]
extern crate lazy_static;

use std::sync::*;
use std::marker::PhantomData;


pub mod domain;
pub mod tables;
pub mod property;
pub mod intern;
pub mod columns;
pub mod joincore;


/**
 * Trait that all storable types must implement.
 *
 * Types that implement this trait should also not implement `Drop`, although this is not yet
 * expressable, and is not presently required.
 * */
pub trait Storable: Sync + Copy + Sized /* + !Drop */ {}
impl<T> Storable for T where T: Sync + Copy + Sized /* + !Drop */ {}


pub type GuardedUniverse = Arc<RwLock<Universe>>;

pub use domain::DomainName;
use domain::MaybeDomain;
use tables::{GetTableName, GenericRowId};

/**
 * A context object whose reference should be passed around everywhere.
 * */
pub struct Universe {
    // FIXME: Tables should have domains
    // FIXME: Tables should be in Arcs.
    //  - allows table links
    //  - probably add 'struct Domain { tables: Vec, properties: Vec }'
    //  Actually it's the 'Domain' that should be in an Arc, not the table.
    pub domains: Vec<MaybeDomain>,
}
impl Universe {
    pub fn new(domains: &[DomainName]) -> Universe {
        Universe {
            domains: Self::get_domains(domains),
        }
    }

    pub fn guard(self) -> GuardedUniverse { Arc::new(RwLock::new(self)) }

    /**
     * Returns a string describing all the tables in the Universe. (But does not include their
     * contents.)
     * */
    pub fn info(&self) -> String {
        let mut out = "".to_owned();
        for domain in &self.domains {
            let domain = match *domain {
                MaybeDomain::Unset(_) => continue,
                MaybeDomain::Domain(ref i) => i,
            };
            use itertools::Itertools;
            let info: String = domain.tables.iter().map(|(_, table)| {
                table.read().unwrap().info()
            }).join(" ");
            out += &format!("{}: {}\n", domain.name, info);
        }
        out
    }
}
use std::fmt;
impl fmt::Debug for Universe {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Universe:")?;
        for domain in &self.domains {
            writeln!(f, "\t{:?}", domain)?;
        }
        write!(f, "")
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
impl<I: PrimInt + ::num_traits::ToPrimitive, T: GetTableName> Iterator for RowIdIterator<I, T> {
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

/// Events that occur on a table with change tracking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event<R> {
    /// A row was added to the table. This could happen as the result of a new Row being pushed,
    /// but it could also be due to a row in a sparse table being reclaimed.
    Create(R),
    /// A row was moved. This doesn't necessarily mean that `dst` was deleted!
    /// Any foreign references to `src` should be changed to `dst`.
    /// The old row at `dst` has been invalidated (by a `Delete` or another `Move`) by the time
    /// this event has been reached. It would be strange to do any other semantic changes.
    Move { src: R, dst: R },
    /// This row is no longer valid. To maintain validity, delete any foreign rows
    /// referencing this row, or have them reference something else.
    Delete(R),
    /// Every row was deleted.
    ClearAll,
}
