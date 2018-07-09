use std::mem;
use std::sync::{Arc, RwLock};
use Universe;
use tables::GetTableName;
use index::GenericRowId;


/// `Tracker`s are notified of structural changes to tables. This requires the 'consistent'
/// guarantee, which is provided by `#[kind = "public"]`.
// FIXME: https://github.com/rust-lang/rust/issues/29628
// https://doc.rust-lang.org/beta/unstable-book/language-features/on-unimplemented.html
// #[rustc_on_unimplemented = "You must implement `Tracker` on `{Self}` so that it can react
// to structural changes in the `#[foreign]` table."]
pub trait Tracker: 'static + Send + Sync {
    type Table: GetTableName;

    /// The foreign table was cleared. Clearing the local table is likely appropriate.
    fn cleared(&mut self, universe: &Universe);

    /// The indicated foreign rows have been deleted or added.
    ///
    /// If the column has an `#[index]`, you can call `$table.track_$col_events(deleted)`.
    /// `added` rows must be processed *after* deleted rows.
    /// Or, if the table is `#[kind = "sorted"]` and has a `#[sort_key]` column, you can call
    /// `$table.track_sorted_$col_events(deleted)`.
    ///
    /// Unfortunately `usize`s are passed instead of `$table::RowId`.
    /// They can be converted using `$table::RowId::from_usize`.
    /// This might be fixed in the future.
    ///
    /// You may lock the foreign table for editing, but making structural changes will likely
    /// cause you trouble.
    ///
    /// If the foreign key is `#[sort_key]`, then the events are sorted. Otherwise, the order is
    /// undefined.
    ///
    /// Ignoring `added` is very typical.
    fn track(&mut self, universe: &Universe, deleted: &[GenericRowId<Self::Table>], added: &[GenericRowId<Self::Table>]);
}
// FIXME: Currently we use &mut, but I've only ever used a unit struct. It is *possibly* useful...


#[doc(hidden)]
pub struct Flush<I: GetTableName> {
    // All the other fields don't need locks, but this one does because we need to continue holding
    // it after releasing the lock on `GenericTable`.
    // We manage borrowing on the other stuff via mem::swap
    trackers: Arc<RwLock<Vec<Box<Tracker<Table=I>>>>>,
    trackers_is_empty: bool, // don't want to lock!
    sort_events: bool,

    del: Vec<GenericRowId<I>>,
    add: Vec<GenericRowId<I>>,
    cleared: bool,
}
impl<I: GetTableName> Default for Flush<I> {
    fn default() -> Self {
        Flush {
            trackers: Default::default(),
            trackers_is_empty: true,
            sort_events: false,

            del: Default::default(),
            add: Default::default(),
            cleared: false,
        }
    }
}
#[doc(hidden)]
impl<I: GetTableName> Flush<I> {
    /// Swap the values out, returning them in a temporary `Flush` that is used for doing actual
    /// flushing. This is required because we need to release the lock on Table to flush, in case
    /// someone wants it.
    pub fn extract(&mut self) -> Self {
        let new = Flush {
            trackers: self.trackers.clone(),
            trackers_is_empty: self.trackers_is_empty,
            sort_events: self.sort_events,

            del: vec![],
            add: vec![],
            cleared: false,
        };
        mem::replace(self, new)
    }

    pub fn flush(&mut self, universe: &Universe) {
        let mut trackers = self.trackers.write().unwrap();
        if self.sort_events {
            self.del.sort();
            self.add.sort();
        }
        for tracker in trackers.iter_mut() {
            if self.cleared {
                tracker.cleared(universe);
            }
            tracker.track(universe, &self.del[..], &self.add[..]);
        }
        self.del.clear();
        self.add.clear();
    }

    /// Return values from a `Flush::extract`. This accomplishes two things:
    /// 1. Conserves the allocated objects
    /// 2. Does the right thing if events have happened in the meantime.
    pub fn restore(&mut self, mut orig: Self) {
        mem::swap(&mut self.trackers, &mut orig.trackers);
        fn swap_vecs<I>(my: &mut Vec<I>, orig: &mut Vec<I>) {
            if my.is_empty() {
                mem::swap(my, orig);
            }
        }
        swap_vecs(&mut self.del, &mut orig.del);
        swap_vecs(&mut self.add, &mut orig.add);
    }

    pub fn need_flush(&self) -> bool {
        if self.trackers_is_empty { return false; }
        !(self.del.is_empty() && self.add.is_empty()) || self.cleared
    }

    pub fn summary(&self) -> String {
        format!("del: {}, add: {}, cleared: {}",
                self.del.len(), self.add.len(), self.cleared)
    }

    pub fn register_tracker<R: Tracker<Table=I>>(&mut self, tracker: R, sort_events: bool) {
        if !R::Table::get_guarantee().consistent {
            panic!("Tried to add tracker to inconsistent table, {}/{}",
                   R::Table::get_domain(), R::Table::get_name());
        }
        let mut trackers = self.trackers.write().unwrap();
        trackers.push(Box::new(tracker));
        self.trackers_is_empty = false;
        self.sort_events |= sort_events;
    }

    /*
    pub fn remove_tracker<R: Tracker<Table=I>>(&mut self) -> Option<Box<R>> {
        let mut trackers = self.trackers.write().unwrap();
        for i in (0..trackers.len()).rev() {
            if trackers[i].downcast_ref::<R>().is_none() { continue; }
            return Some(trackers.remove(i));
        }
        None
    }
    */

    pub fn trackers_is_empty(&self) -> bool { self.trackers_is_empty }
}
/// $Table notifies us of events via this impl.
#[doc(hidden)]
impl<I: GetTableName> Flush<I> {
    #[inline]
    pub fn cleared(&mut self) {
        if self.need_flush() {
            panic!("cleared(), but there are still pending events!");
        }
        self.cleared = true;
        self.add.clear();
        self.del.clear();
    }

    #[inline] pub fn add(&mut self, i: GenericRowId<I>) { self.add.push(i) }
    #[inline] pub fn del(&mut self, i: GenericRowId<I>) { self.del.push(i) }
    #[inline] pub fn add_reserve(&mut self, n: usize) { self.add.reserve(n) }
    #[inline] pub fn del_reserve(&mut self, n: usize) { self.del.reserve(n) }
}
