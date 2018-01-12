//! `GenericRowId`, `CheckedRowId`, and `RowRange`.
// FIXME: Need https://github.com/rust-lang/rust/issues/38078 to ditch obnoxious verbosity
// FIXME: s/GenericRowId/WillCheckRowId

use std::marker::PhantomData;
use std::fmt;
use tables::{GetTableName, LockedTable};
use num_traits::{ToPrimitive, One, Bounded};
use num_traits::cast::FromPrimitive;

use Universe;
use tracking::Tracker;

// #[derive] nothing; stupid phantom data...
pub struct GenericRowId<T: GetTableName> {
    #[doc(hidden)]
    pub i: T::Idx,
    #[doc(hidden)]
    pub t: PhantomData<T>,
}
impl<T: GetTableName> GenericRowId<T> {
    pub fn new(i: T::Idx) -> Self {
        GenericRowId {
            i,
            t: PhantomData,
        }
    }

    pub fn from_usize(i: usize) -> Self where T::Idx: FromPrimitive {
        Self::new(T::Idx::from_usize(i).unwrap())
    }
    pub fn to_usize(&self) -> usize { self.i.to_usize().unwrap() }
    pub fn to_raw(&self) -> T::Idx { self.i }

    pub fn next(&self) -> Self {
        Self::new(self.i + T::Idx::one())
    }

    pub fn register_tracker(universe: &Universe, t: Box<Tracker + Send + Sync>) {
        let gt = universe.get_generic_table(T::get_domain().get_id(), T::get_name());
        let mut gt = gt.write().unwrap();
        gt.add_tracker(t);
    }

    pub fn get_domain() -> ::domain::DomainName { T::get_domain() }
    pub fn get_name() -> ::tables::TableName { T::get_name() }
}
impl<T: GetTableName> Default for GenericRowId<T> {
    fn default() -> Self {
        GenericRowId {
            i: T::Idx::max_value() /* UNDEFINED_INDEX */,
            t: PhantomData,
        }
    }
}
impl<T: GetTableName> Clone for GenericRowId<T> {
    fn clone(&self) -> Self {
        Self::new(self.i)
    }
}
impl<T: GetTableName> Copy for GenericRowId<T> { }

impl<T: GetTableName> fmt::Debug for GenericRowId<T>
where T::Idx: fmt::Display
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}[{}]", T::get_name().0, self.i)
    }
}

/// This value can be used to index into table columns.
/// It borrows the table to ensure that it is a valid index.
/// It has already been checked.
#[derive(Hash, PartialOrd, Ord, Eq, PartialEq)]
pub struct CheckedRowId<'a, T: LockedTable + 'a> {
    i: <T::Row as GetTableName>::Idx,
    // FIXME: This should be a PhantomData. NBD since these things are short-lived.
    table: &'a T,
}
impl<'a, T: LockedTable + 'a> Clone for CheckedRowId<'a, T> where <T::Row as GetTableName>::Idx: Copy {
    fn clone(&self) -> Self {
        Self { i: self.i, table: self.table }
    }
}
impl<'a, T: LockedTable + 'a> Copy for CheckedRowId<'a, T> where <T::Row as GetTableName>::Idx: Copy {}
impl<'a, T: LockedTable + 'a> CheckedRowId<'a, T> {
    /// Create a `CheckedRowId` without actually checking.
    pub unsafe fn fab(i: <T::Row as GetTableName>::Idx, table: &'a T) -> Self {
        Self { i, table }
    }
    pub fn to_usize(&self) -> usize { self.i.to_usize().unwrap() }
    pub fn to_raw(&self) -> <T::Row as GetTableName>::Idx { self.i }
    pub fn next(self) -> GenericRowId<T::Row> { self.uncheck().next() }
}
impl<'a, T: LockedTable + 'a> fmt::Debug for CheckedRowId<'a, T>
where <T::Row as GetTableName>::Idx: fmt::Debug
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}[{}]", T::Row::get_name().0, self.i)
    }
}

#[test]
#[cfg(test)]
fn test_formatting() {
    use tables::TableName;
    struct TestName;
    impl GetTableName for TestName {
        type Idx = usize;
        fn get_name() -> TableName { TableName("test_table") }
    }
    let gen: GenericRowId<TestName> = GenericRowId {
        i: 23,
        t: ::std::marker::PhantomData,
    };
    assert_eq!("test_table[23]", format!("{:?}", gen));
}


use std::cmp::{Eq, PartialEq, PartialOrd, Ord};
impl<T: GetTableName> PartialEq for GenericRowId<T> {
    fn eq(&self, other: &GenericRowId<T>) -> bool {
        self.i.eq(&other.i)
    }
}
impl<T: GetTableName> PartialOrd for GenericRowId<T> {
    fn partial_cmp(&self, other: &GenericRowId<T>) -> Option<::std::cmp::Ordering> {
        self.i.partial_cmp(&other.i)
    }
}
impl<T: GetTableName> Ord for GenericRowId<T> {
    fn cmp(&self, other: &GenericRowId<T>) -> ::std::cmp::Ordering {
        self.i.cmp(&other.i)
    }
}

impl<T: GetTableName> Eq for GenericRowId<T> {}

// Things get displeasingly manual due to the PhantomData.
// CheckedRowId can derive hash, and is unserializable.
use std::hash::{Hash, Hasher};
impl<T: GetTableName> Hash for GenericRowId<T>
where T::Idx: Hash
{
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.i.hash(state);
    }
}

use rustc_serialize::{Encoder, Encodable, Decoder, Decodable};
impl<T: GetTableName> Encodable for GenericRowId<T>
where T::Idx: Encodable
{
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        self.i.encode(s)
    }
}

impl<T: GetTableName> Decodable for GenericRowId<T>
where T::Idx: Decodable
{
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        Ok(Self::new(T::Idx::decode(d)?))
    }
}




#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[derive(RustcDecodable, RustcEncodable)]
pub struct RowRange<R> {
    pub start: R,
    pub end: R,
}
use std::ops::Range;
impl<R> Into<Range<R>> for RowRange<R> {
    fn into(self) -> Range<R> {
        self.start..self.end
    }
}
impl<R> From<Range<R>> for RowRange<R> {
    fn from(range: Range<R>) -> RowRange<R> {
        RowRange {
            start: range.start,
            end: range.end,
        }
    }
}
impl<T: GetTableName> RowRange<GenericRowId<T>> {
    pub fn empty() -> Self {
        RowRange {
            start: GenericRowId::default(),
            end: GenericRowId::default(),
        }
    }

    /// Return the `n`th row after the start, if it is within the range.
    pub fn offset(&self, n: T::Idx) -> Option<GenericRowId<T>> {
        use num_traits::CheckedAdd;
        let at = self.start.to_raw().checked_add(&n);
        let at = if let Some(at) = at {
            at
        } else {
            return None;
        };
        if at > self.end.to_raw() {
            None
        } else {
            Some(GenericRowId::new(at))
        }
    }

    /// Return how many rows are in this range.
    pub fn len(&self) -> usize {
        self.end.to_usize() - self.start.to_usize()
    }

    /// Return `true` if the given row is within this range.
    pub fn contains(&self, o: GenericRowId<T>) -> bool {
        self.start <= o && o < self.end
    }

    /// If the given row is within this RowRange, return its offset from the beginning.
    pub fn inner_index(&self, o: GenericRowId<T>) -> Option<T::Idx> {
        if self.contains(o) {
            Some(o.to_raw() - self.start.to_raw())
        } else {
            None
        }
    }

    pub fn iter_slow(&self) -> UncheckedIter<T> {
        UncheckedIter {
            i: self.start.to_raw(),
            end: self.end.to_raw(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CheckedIter<'a, T: LockedTable + 'a> {
    table: &'a T,
    i: <T::Row as GetTableName>::Idx,
    end: <T::Row as GetTableName>::Idx,
}
impl<'a, T: LockedTable> CheckedIter<'a, T> {
    pub fn from(table: &'a T, slice: RowRange<GenericRowId<T::Row>>) -> Self {
        assert!(slice.start.to_usize() < table.len());
        assert!(slice.end.to_usize() <= table.len()); // Remember: end is excluded from the iteration!
        CheckedIter {
            table,
            i: slice.start.to_raw(),
            end: slice.end.to_raw(),
        }
    }
}
impl<'a, T: LockedTable> Iterator for CheckedIter<'a, T> {
    type Item = CheckedRowId<'a, T>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.end {
            None
        } else {
            let ret = CheckedRowId {
                i: self.i,
                table: self.table,
            };
            self.i = ret.next().i;
            Some(ret)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let s = self.end.to_usize().unwrap() - self.i.to_usize().unwrap();
        (s, Some(s))
    }
}

pub struct UncheckedIter<T: GetTableName> {
    i: T::Idx,
    end: T::Idx,
}
impl<T: GetTableName> Iterator for UncheckedIter<T> {
    type Item = GenericRowId<T>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.end {
            None
        } else {
            let ret = GenericRowId {
                i: self.i,
                t: PhantomData,
            };
            self.i = ret.next().i;
            Some(ret)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let s = self.end.to_usize().unwrap() - self.i.to_usize().unwrap();
        (s, Some(s))
    }
}




pub trait Checkable {
    type Row: GetTableName;
    fn check<'a, L>(self, table: &'a L) -> CheckedRowId<'a, L>
    where L: LockedTable<Row=Self::Row>;
    fn uncheck(self) -> GenericRowId<Self::Row>;
}
impl<T: GetTableName> Checkable for GenericRowId<T> {
    type Row = T;
    fn check<L>(self, table: &L) -> CheckedRowId<L>
    where L: LockedTable<Row=Self::Row>
    {
        let i = self.i;
        if i.to_usize().unwrap() >= table.len() {
            panic!("index out of bounds on table {}: the len is {}, but the index is {}",
                   T::get_name(), table.len(), i);
        }
        unsafe {
            CheckedRowId::fab(i, table)
        }
    }

    fn uncheck(self) -> GenericRowId<T> { self }
}
impl<'a, T: LockedTable + 'a> Checkable for CheckedRowId<'a, T> {
    type Row = T::Row;
    fn check<'c, L>(self, table: &'c L) -> CheckedRowId<'c, L>
    where L: LockedTable<Row=Self::Row>
    {
        if cfg!(debug) && self.table as *const T as usize != table as *const L as usize {
            panic!("mismatched tables");
        }
        CheckedRowId {
            i: self.i,
            table,
        }
    }
    fn uncheck(self) -> GenericRowId<T::Row> { GenericRowId::new(self.i) }
}


use ::joincore::{JoinCore, Join};
use std::collections::btree_map;
/// A `CheckedIter` that skips rows marked for deletion.
pub struct ConsistentIter<'a, T: LockedTable + 'a> {
    rows: CheckedIter<'a, T>,
    deleted: JoinCore<btree_map::Keys<'a, usize, ()>>,
}
impl<'a, T: LockedTable + 'a> ConsistentIter<'a, T> {
    pub fn new(rows: CheckedIter<'a, T>, deleted: &'a btree_map::BTreeMap<usize, ()>) -> Self {
        Self {
            rows,
            deleted: JoinCore::new(deleted.keys()),
        }
    }
}
impl<'a, T: LockedTable + 'a> Iterator for ConsistentIter<'a, T> {
    type Item = CheckedRowId<'a, T>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(row) = self.rows.next() {
            match self.deleted.join(row.to_usize(), |l, r| l.cmp(r)) {
                // This join is a bit backwards.
                Join::Next | Join::Stop => return Some(row),
                Join::Match(_) => continue,
            }
        }
        None
    }
}
