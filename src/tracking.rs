use std::mem;
use std::sync::{Arc, RwLock};
use Universe;
use tables::GetTableName;
use index::GenericRowId;

/// Everything you need to define a [`Tracker`].
pub mod prelude {
    pub use ::Universe;
    pub use ::tracking::{Tracker, SelectRows};
    pub use ::event::{self, Event};
}

/// Helper trait used to a parameter of a parameterized type.
pub trait GetParam { type T; }
impl<T: GetTableName> GetParam for GenericRowId<T> { type T = T; }

/// Indicates whether all rows hae been selected, or only some of them.
/// (No selection is indicated by not receiving a call.)
#[derive(Debug, Clone)]
pub enum Select<I> {
    All,
    These(I),
}
impl<I> Select<I> {
    pub fn map<F, R>(self, f: F) -> Select<R>
    where F: FnOnce(I) -> R
    {
        match self {
            Select::All => Select::All,
            Select::These(rows) => Select::These(f(rows)),
        }
    }

    pub fn as_ref(&self) -> Select<&I> {
        match self {
            Select::All => Select::All,
            Select::These(rows) => Select::These(rows),
        }
    }

    pub fn as_mut(&mut self) -> Select<&mut I> {
        match self {
            Select::All => Select::All,
            Select::These(rows) => Select::These(rows),
        }
    }
}
impl<'a, T: GetTableName> Select<&'a GenericRowId<T>> {
    // FIXME: iter?
}
pub type SelectRows<'a, T> = Select<&'a [GenericRowId<T>]>;
pub type SelectAny<'a> = Select<::any_slice::AnySliceRef<'a>>;

use event::Event;

/// `Tracker`s are notified of structural changes to tables. This requires the 'consistent'
/// guarantee on the foreign table, which is provided by `#[kind = "consistent"]`.
/// You use `#[foreign_auto]` to derive an implementation.
// FIXME: https://github.com/rust-lang/rust/issues/29628
// https://doc.rust-lang.org/beta/unstable-book/language-features/on-unimplemented.html
// #[rustc_on_unimplemented = "You must implement `Tracker` on `{Self}` so that it can react
// to structural changes in the `#[foreign]` table."]
pub trait Tracker: 'static + Send + Sync {
    /// `$foreign_table::Row`.
    type Foreign: GetTableName;

    /// Indicate if the Tracker is interested in the given [`Event`] type.
    /// A typical implementation is `event.is_removal`.
    fn consider(&self, event: Event) -> bool;

    /// If this returns `true`, then the rows given to `handle` will be sorted.
    /// Otherwise, the order is undefined.
    fn sort(&self) -> bool;

    /// Something has happened to the indicated foreign rows.
    ///
    /// You may lock the foreign table for editing, but making structural changes will likely
    /// cause you trouble.
    ///
    /// # Deletion
    /// The most common behavior of a tracker is to respond to deletion events.
    ///
    /// If the column has an `#[index]`, you can call `$table.track_$col_events(deleted)`.
    /// Or, if the table is `#[kind = "sorted"]` and has a `#[sort_key]` column, you can call
    /// `$table.track_sorted_$col_events(deleted)`.
    ///
    /// # Just-in-time
    /// Any deleted rows in the foreign table will still be valid (at least so far as you'll be
    /// able to access their contents without error), but will become actually-deleted after the
    /// flush completes.
    ///
    /// Any newly created rows are valid.
    fn handle(&mut self, universe: &Universe, event: Event, rows: SelectRows<Self::Foreign>);
    // All of my impls of `Tracker` have been unit structs so far, so &mut seems a bit silly.
    // But someone may find a use for it.
}


#[doc(hidden)]
pub struct Flush<I: GetTableName> {
    // All the other fields don't need locks, but this one does because we need to continue holding
    // it after releasing the lock on `GenericTable`.
    // We manage borrowing on the other stuff via mem::swap
    trackers: Arc<RwLock<Vec<Box<Tracker<Foreign=I>>>>>,
    trackers_is_empty: bool, // don't want to lock!

    selected: Vec<GenericRowId<I>>,
    select_all: bool,
}
impl<I: GetTableName> Default for Flush<I> {
    fn default() -> Self {
        Flush {
            trackers: Default::default(),
            trackers_is_empty: true,

            selected: vec![],
            select_all: false,
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

            selected: vec![],
            select_all: false,
        };
        mem::replace(self, new)
    }

    fn selection(&self) -> SelectRows<I> {
        if self.select_all {
            Select::All
        } else {
            Select::These(&self.selected[..])
        }
    }

    pub fn reserve(&mut self, n: usize) { self.selected.reserve(n) }

    pub fn flush(&mut self, universe: &Universe, event: Event) {
        let mut sorted = false;

        {
            let mut trackers = self.trackers.write().unwrap();
            for tracker in trackers.iter_mut() {
                if !tracker.consider(event) { continue; }
                if !sorted && tracker.sort() {
                    sorted = true;
                    self.selected.sort();
                }
                tracker.handle(universe, event, self.selection());
            }
        }
        {
            let fallback = universe.event_handlers.get(event);
            let gt = I::get_generic_table(universe);
            let mut gt = gt.write().unwrap();
            let gt = &mut *gt;
            if !sorted && fallback.needs_sort(gt) {
                self.selected.sort();
            }
            let rows = self
                .selection()
                .as_ref()
                .map(|i| ::any_slice::AnySliceRef::from(i));
            fallback.handle(universe, gt, event, rows);
        }

        self.selected.clear();
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
        swap_vecs(&mut self.selected, &mut orig.selected);
    }

    pub fn need_flush(&self) -> bool {
        if self.trackers_is_empty { return false; }
        !self.selected.is_empty() || self.select_all
    }

    pub fn summary(&self) -> String {
        format!("selected: {}, select_all: {}",
                self.selected.len(), self.select_all)
    }

    pub fn register_tracker<R: Tracker<Foreign=I>>(&mut self, tracker: R) {
        if !R::Foreign::get_guarantee().consistent {
            panic!("Tried to add tracker to inconsistent table, {}/{}",
                   R::Foreign::get_domain(), R::Foreign::get_name());
        }
        let mut trackers = self.trackers.write().unwrap();
        trackers.push(Box::new(tracker));
        self.trackers_is_empty = false;
    }

    pub fn trackers_is_empty(&self) -> bool { self.trackers_is_empty }
}
/// $Table notifies us of events via this impl.
#[doc(hidden)]
impl<I: GetTableName> Flush<I> {
    #[inline]
    pub fn select_all(&mut self) {
        self.select_all = true;
        self.selected.clear();
    }

    #[inline]
    pub fn select(&mut self, i: GenericRowId<I>) {
        self.selected.push(i);
    }
}



impl Universe {
    /// Add a custom tracker.
    /// You'll typically use this to maintain consistentcy with non-table data structures.
    /// For tables you'll generally use `#[foreign]` to be provided a struct to implement
    /// [`Tracker`] on. Such trackers are automatically added to each table instance; this
    /// function adds the tracker only to a particular instance.
    pub fn register_tracker<R: Tracker>(&self, tracker: R) {
        let gt = <R::Foreign as GetTableName>::get_generic_table(self);
        let mut gt = gt.write().unwrap();
        let flush = gt.table.get_flush();
        let flush: &mut Flush<R::Foreign> = flush.downcast_mut().expect("wrong foreign table type");
        flush.register_tracker(tracker)
    }
}
