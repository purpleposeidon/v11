use std::mem;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
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

/// Indicates whether all rows have been selected, or only some of them.
/// (No selection is indicated by not receiving a call.)
#[derive(Debug, Clone)]
#[derive(Serialize, Deserialize)]
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

    pub fn unwrap(self) -> I {
        match self {
            Select::All => panic!("unwrap"),
            Select::These(i) => i,
        }
    }
}
impl<'a, T: GetTableName> Select<&'a [GenericRowId<T>]> {
    /// Returns an iterator over the `Select::These` rows,
    /// or use the given range if all rows were selected.
    pub fn iter_or_all<I>(self, all: I) -> SelectIter<'a, I, T>
    where I: Iterator<Item=GenericRowId<T>>
    {
        self.iter_or_all_with(|| all)
    }

    pub fn iter_or_all_with<F, I>(self, f: F) -> SelectIter<'a, I, T>
    where
        F: FnOnce() -> I,
        I: Iterator<Item=GenericRowId<T>>,
    {
        match self {
            Select::All => SelectIter::All(f()),
            Select::These(rows) => SelectIter::These(rows.iter()),
        }
    }
}
impl<T: GetTableName> Select<Vec<GenericRowId<T>>> {
    pub fn as_slice<'a>(&'a self) -> SelectRows<'a, T> {
        self
            .as_ref()
            .map(|rows| rows.as_slice())
    }
}


pub type SelectRows<'a, T> = Select<&'a [GenericRowId<T>]>;
pub type SelectOwned<T> = Select<Vec<GenericRowId<T>>>;
pub type SelectAny<'a> = Select<::any_slice::AnySliceRef<'a>>;

pub enum SelectIter<'a, I, T>
where
    I: Iterator<Item=GenericRowId<T>>,
    T: GetTableName,
{
    All(I),
    These(::std::slice::Iter<'a, GenericRowId<T>>),
}
impl<'a, I, T> Iterator for SelectIter<'a, I, T>
where
    I: Iterator<Item=GenericRowId<T>>,
    T: GetTableName,
{
    type Item = GenericRowId<T>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            SelectIter::All(r) => r.next(),
            SelectIter::These(r) => r.next().map(|i| *i),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            SelectIter::All(r) => r.size_hint(),
            SelectIter::These(r) => r.size_hint(),
        }
    }
}

use event::Event;

/// `Tracker`s are notified of structural changes to tables. This requires the 'consistent'
/// guarantee on the foreign table, which is provided by `#[kind = "consistent"]`.
/// You can use `#[foreign_auto]` to derive an implementation.
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
    fn handle(&self, universe: &Universe, event: Event, rows: SelectRows<Self::Foreign>);
}


#[doc(hidden)]
pub struct Flush<T: GetTableName> {
    // All the other fields don't need locks, but this one does because we need to continue holding
    // it after releasing the lock on `GenericTable`.
    // We manage borrowing on the other stuff via mem::swap
    trackers: Arc<RwLock<Vec<Box<Tracker<Foreign=T>>>>>,
    trackers_is_empty: bool, // don't want to lock!

    selected: Vec<GenericRowId<T>>,
    select_all: bool,

    remapped: HashMap<GenericRowId<T>, GenericRowId<T>>,
}
use std::fmt;
impl<T: GetTableName> fmt::Debug for Flush<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "<Flush>")
    }
}
impl<T: GetTableName> Default for Flush<T> {
    fn default() -> Self {
        Flush {
            trackers: Default::default(),
            trackers_is_empty: true,

            selected: vec![],
            select_all: false,

            remapped: HashMap::new(),
        }
    }
}
#[doc(hidden)]
impl<T: GetTableName> Flush<T> {
    /// Swap the values out, returning them in a temporary `Flush` that is used for doing actual
    /// flushing. This is required because we need to release the lock on Table to flush, in case
    /// someone wants it.
    pub fn extract(&mut self) -> Self {
        let new = Flush {
            trackers: self.trackers.clone(),
            trackers_is_empty: self.trackers_is_empty,

            selected: vec![],
            select_all: false,

            remapped: mem::replace(&mut self.remapped, HashMap::new()),
        };
        mem::replace(self, new)
    }

    fn selection(&self) -> SelectRows<T> {
        if self.select_all {
            Select::All
        } else {
            Select::These(&self.selected[..])
        }
    }

    pub fn set_remapping(&mut self, remap: &[(GenericRowId<T>, GenericRowId<T>)]) {
        // FIXME: Mapping never gets reset; kind of a memory leak but not super serious.
        self.remapped.clear();
        let remap = remap
            .iter()
            .map(|&(o, n)| (o, n));
        self
            .remapped
            .extend(remap);
    }

    pub fn remap(&self, old: GenericRowId<T>) -> Option<GenericRowId<T>> {
        self
            .remapped
            .get(&old)
            .map(|&i| i)
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
            let gt = T::get_generic_table(universe);
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

    pub fn flush_all(
        &self,
        universe: &Universe,
        event: Event,
    ) {
        {
            let trackers = self.trackers.read().unwrap();
            for tracker in trackers.iter() {
                if !tracker.consider(event) { continue; }
                tracker.handle(universe, event, Select::All);
            }
        }
        {
            let fallback = universe.event_handlers.get(event);
            let gt = T::get_generic_table(universe);
            fallback.handle(universe, gt, event, Select::All);
        }
    }
    pub fn flush_selection<I>(
        &self,
        universe: &Universe,
        event: Event,
        selection_sorted: bool,
        selection: I,
    )
    where
        I: Iterator<Item=GenericRowId<T>>,
    {
        let assert_nosort = |b| if b && !selection_sorted {
            panic!("Tracker requires sorted a sorted selection, but the selection can not be sorted");
        };

        let mut collection = Vec::new();
        let mut selection = Some(selection);
        macro_rules! select {
            () => {{
                if let Some(selection) = selection.take() {
                    collection.extend(selection);
                }
                Select::These(collection.as_slice())
            }};
        }

        {
            let trackers = self.trackers.read().unwrap();
            for tracker in trackers.iter() {
                if !tracker.consider(event) { continue; }
                assert_nosort(tracker.sort());
                tracker.handle(universe, event, select!());
            }
        }
        {
            let fallback = universe.event_handlers.get(event);
            let gt = T::get_generic_table(universe);
            assert_nosort(fallback.needs_sort(gt));
            let rows = select!();
            let rows = rows
                .as_ref()
                .map(|i| ::any_slice::AnySliceRef::from(i));
            fallback.handle(universe, gt, event, rows);
        }
    }

    /// Return values from a `Flush::extract`. This accomplishes two things:
    /// 1. Conserves the allocated objects
    /// 2. Does the right thing if events have happened in the meantime.
    pub fn restore(&mut self, mut orig: Self) {
        // FIXME: Suppose we both have remappings. Which do we keep?
        mem::swap(&mut self.trackers, &mut orig.trackers);
        fn swap_vecs<T>(my: &mut Vec<T>, orig: &mut Vec<T>) {
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

    pub fn register_tracker<R: Tracker<Foreign=T>>(&mut self, tracker: R) {
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
impl<T: GetTableName> Flush<T> {
    #[inline]
    pub fn select(&mut self, i: GenericRowId<T>) {
        self.selected.push(i);
    }

    #[inline]
    pub fn select_all(&mut self) {
        self.select_all = true;
        self.selected.clear();
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
        let flush = gt.table.get_flush_mut();
        let flush: &mut Flush<R::Foreign> = flush.downcast_mut().expect("wrong foreign table type");
        flush.register_tracker(tracker)
    }
}
