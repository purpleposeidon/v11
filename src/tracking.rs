/// `Tracker`s are notified of structural changes to tables. This requires the 'consistent'
/// guarantee, which is provided by `#[kind = "public"]`.
// FIXME: https://github.com/rust-lang/rust/issues/29628
// https://doc.rust-lang.org/beta/unstable-book/language-features/on-unimplemented.html
// #[rustc_on_unimplemented = "You must implement `Tracker` on `{Self}` so that it can react
// to structural changes in the `#[foreign]` table."]
pub trait Tracker {
    /// The foreign table was cleared. Clearing the local table is likely appropriate.
    fn cleared(&mut self, universe: &Universe);

    /// The indicated foreign rows have been deleted or added.
    ///
    /// If the column has an `#[index]`, you can call `$table.track_$col_events(deleted)`.
    /// `added` rows must be processed *after* deleted rows.
    ///
    /// Unfortunately `usize`s are passed instead of `$table::RowId`.
    /// They can be converted using `$table::RowId::from_usize`.
    /// This might be fixed in the future.
    ///
    /// You may lock the foreign table for editing, but making structural changes would likely
    /// cause you trouble.
    fn track(&mut self, universe: &Universe, deleted: &[usize], added: &[usize]);

    // FIXME: usize instead of GenericRowId.
    // This'd break object safety tho... GenericTable has:
    //  - deleted: Vec<usize>, 
    //  - Vec<Box<Tracker>>
    // Might need to box a tracker container trait.
}

use std::sync::{Arc, RwLock};
use std::mem;
use tables::GenericTable;
use Universe;

#[doc(hidden)]
pub struct GenericFlush {
    trackers: Arc<RwLock<Vec<Box<Tracker + Send + Sync>>>>,
    delete: Vec<usize>,
    add: Vec<usize>,
    cleared: bool,
}
#[doc(hidden)]
#[must_use]
impl GenericFlush {
    pub fn flush(&mut self, universe: &Universe) {
        let mut trackers = self.trackers.write().unwrap();
        for tracker in trackers.iter_mut() {
            if self.cleared {
                tracker.cleared(universe);
            }
            tracker.track(universe, &self.delete[..], &self.add[..]);
        }
    }

    /// Return the Vecs to the GT to save on allocations. Best-effort.
    pub fn restore(mut self, gt: &mut GenericTable) {
        assert!(gt.delete.is_empty()); // or just early-return
        assert!(gt.add.is_empty());
        mem::swap(&mut gt.delete, &mut self.delete);
        mem::swap(&mut gt.add, &mut self.add);
    }
}

/// Events and their `Tracker`s.
#[doc(hidden)]
impl GenericTable {
    pub fn skip_flush(&self) -> bool {
        self.delete.is_empty() && self.add.is_empty() && !self.cleared && !self.need_flush
    }

    pub fn acquire_flush(&mut self) -> GenericFlush {
        let mut delete = Vec::new();
        let mut add = Vec::new();
        mem::swap(&mut delete, &mut self.delete);
        mem::swap(&mut add, &mut self.add);
        let cleared = self.cleared;
        self.cleared = false;
        self.need_flush = false;
        GenericFlush {
            trackers: self.trackers.clone(),
            delete,
            add,
            cleared,
        }
    }

    pub fn cleared(&mut self) {
        self.cleared = true;
        self.need_flush = true;
        self.add.clear();
        self.delete.clear();
        self.free.clear();
    }

    pub fn add_tracker(&mut self, t: Box<Tracker + Send + Sync>) {
        if !self.guarantee.consistent {
            panic!("Tried to add tracker to inconsistent table, {}/{}", self.domain, self.name);
        }
        self.trackers.write().unwrap().push(t);
        self.no_trackers = false;
    }

    fn skip_events(&self) -> bool { self.no_trackers }

    pub fn delete(&mut self, i: usize) {
        self.free.insert(i, ()); // freelist needs to stay up to date even if nobody's watching
        if self.skip_events() { return; }
        self.need_flush = true;
        self.delete.push(i);
    }

    pub fn add(&mut self, i: usize) {
        if self.skip_events() { return; }
        self.need_flush = true;
        self.add.push(i);
    }

    pub fn delete_reserve(&mut self, n: usize) {
        if self.skip_events() { return; }
        self.delete.reserve(n);
    }

    pub fn add_reserve(&mut self, n: usize) {
        if self.skip_events() { return; }
        self.add.reserve(n);
    }
}
