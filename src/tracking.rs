use std::collections::HashMap;
use crate::Universe;
use crate::tables::GetTableName;
use crate::index::GenericRowId;
use std::sync::{Arc, RwLock};

/// Everything you need to define a [`Tracker`].
pub mod prelude {
    pub use crate::Universe;
    pub use crate::tracking::{Tracker, SelectRows, SelectAny};
    pub use crate::event::{self, Event};
}

/// Helper trait used to a parameter of a parameterized type.
pub trait GetParam { type T; }
impl<T: GetTableName> GetParam for GenericRowId<T> { type T = T; }

/// Indicates whether all rows have been selected, or only some of them.
/// (No selection is indicated by not receiving a call.)
#[derive(Debug, Copy, Clone)]
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

    pub fn is_all(&self) -> bool {
        match self {
            Select::All => true,
            _ => false,
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

    pub fn as_any(&self) -> SelectAny where T: ::std::any::Any + Send + Sync {
        self
            .as_ref()
            .map(|s| {
                crate::any_slice::AnySliceRef::from(s)
            })
    }

    pub fn is_empty(&self) -> bool {
        match self {
            Select::All => false,
            Select::These(ref rows) => rows.is_empty(),
        }
    }

    pub fn to_owned(self) -> SelectOwned<T> {
        match self {
            Select::All => Select::All,
            Select::These(rows) => Select::These(rows.iter().cloned().collect()),
        }
    }
}
impl<T: GetTableName> Select<Vec<GenericRowId<T>>> {
    pub fn as_slice<'a>(&'a self) -> SelectRows<'a, T> {
        self
            .as_ref()
            .map(|rows| rows.as_slice())
    }

    pub fn sort(&mut self) {
        self
            .as_mut()
            .map(|s| s.sort());
    }

    pub fn push(&mut self, row: GenericRowId<T>) {
        self
            .as_mut()
            .map(|s| s.push(row));
    }

    pub fn reserve(&mut self, n: usize) {
        self
            .as_mut()
            .map(|s| s.reserve(n));
    }
}


pub type SelectRows<'a, T> = Select<&'a [GenericRowId<T>]>;
pub type SelectOwned<T> = Select<Vec<GenericRowId<T>>>;
pub type SelectAny<'a> = Select<crate::any_slice::AnySliceRef<'a>>;

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

use crate::event::{self, Event};

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
    /// You probably are tho!
    fn consider(&self, _event: Event) -> bool { true }

    /// If this returns `true`, then the rows given to `handle` will be sorted.
    /// Otherwise, the order is undefined.
    fn sort(&self) -> bool;

    /// Something has happened to the indicated foreign rows.
    /// There are two sorts of actions you may take here:
    /// 1. Propagate the selection to any dependent rows.
    /// 2. Change things under the advisement of the indicated rows.
    ///
    /// # 1. Implementing Propagation
    /// You can use `#[foreign_auto]` to derive the Tracker automatically.
    /// Its implementation looks like this:
    ///
    /// ```ignore
    /// let mut rows = $table::read(universe).select_$column(selected);
    /// handler.run(universe, event, rows);
    /// ```
    ///
    /// # 2. Implementing Changes
    /// In this case, you will want to override `fn consider()`, and you will ignore the `handler`
    /// argument.
    ///
    /// # Just-in-time
    /// Any newly created rows are already valid.
    ///
    /// Any deleted rows in the foreign table will still be valid (at least so far as you'll be
    /// able to access their contents without a panic), but will become actually-deleted after the
    /// flush completes.
    ///
    /// You may lock the foreign table for editing, but making structural changes to it
    /// will likely cause trouble.
    fn handle(
        &self,
        universe: &Universe,
        event: Event,
        rows: SelectRows<Self::Foreign>,
        handler: &dyn event::Function,
    );
}

#[doc(hidden)]
pub type GuardedFlush<T> = Arc<RwLock<Flush<T>>>;

#[doc(hidden)]
pub struct Flush<T: GetTableName> {
    trackers: Vec<Box<Tracker<Foreign=T>>>,
    identity_remapping: bool,
    pub remapped: HashMap<GenericRowId<T>, GenericRowId<T>>,
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
            identity_remapping: false,
            remapped: HashMap::new(),
        }
    }
}
#[doc(hidden)]
impl<T: GetTableName> Flush<T> {
    pub fn do_flush(
        &self,
        universe: &Universe,
        event: Event,
        pushed: bool,
        delete: bool,
        mut select: SelectOwned<T>,
        include_self: bool,
    ) -> SelectOwned<T> {
        if (pushed && delete) || (event.is_removal && event.is_creation) {
            panic!("Can't interleave pushes & deletes");
        }
        // either way, send to trackers first
        let function = universe.event_handlers.get(event);
        let mut sorted = select.is_all();
        {
            for tracker in &self.trackers {
                if !tracker.consider(event) { continue; }
                if !sorted && tracker.sort() {
                    sorted = true;
                    select.as_mut().map(|s| s.sort());
                }
                tracker.handle(
                    universe,
                    event,
                    select.as_slice(),
                    function,
                );
            }
        }
        if include_self {
            let gt = T::get_generic_table(universe);
            if !sorted && function.needs_sort(gt) {
                select.sort();
            }
            let select = select.as_slice();
            let select = select.as_any();
            function.handle(universe, gt, event, select);
        }
        select
    }
    pub fn set_remapping(&mut self, remap: &[(GenericRowId<T>, GenericRowId<T>)]) {
        if self.identity_remapping {
            // FIXME: We ought to panic so that you must be less wasteful.
            return;
        }
        self.remapped.clear();
        let remap = remap
            .iter()
            .map(|&(o, n)| (o, n));
        self
            .remapped
            .extend(remap);
    }
    pub fn remap(&self, old: GenericRowId<T>) -> Option<GenericRowId<T>> {
        if self.identity_remapping { return Some(old); }
        self
            .remapped
            .get(&old)
            .map(|&i| i)
    }
    pub fn has_remapping(&self) -> bool { !self.remapped.is_empty() }
    pub fn set_identity_remap(&mut self) {
        self.identity_remapping = true;
    }


    pub fn register_tracker<R: Tracker<Foreign=T>>(&mut self, tracker: R) {
        if !R::Foreign::get_guarantee().consistent {
            panic!("Tried to add tracker to inconsistent table, {}/{}",
                   R::Foreign::get_domain(), R::Foreign::get_name());
        }
        self.trackers.push(Box::new(tracker));
    }

    pub fn trackers_is_empty(&self) -> bool { self.trackers.is_empty() }
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
        let flush: &mut GuardedFlush<R::Foreign> = flush.downcast_mut().expect("wrong foreign table type");
        let mut flush = flush.write().unwrap();
        flush.register_tracker(tracker)
    }
}
