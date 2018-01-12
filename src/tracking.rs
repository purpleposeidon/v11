
/// `Tracker`s are notified of structural changes to tables. This requires the 'consistent'
/// guarantee, which is provided by `#[kind = "public"]`.
// FIXME: https://github.com/rust-lang/rust/issues/29628
// https://doc.rust-lang.org/beta/unstable-book/language-features/on-unimplemented.html
// #[rustc_on_unimplemented = "You must implement `Tracker` on `{Self}` so that it can react
// to structural changes in the `#[foreign]` table."]
pub trait Tracker {
    fn cleared(&mut self, universe: &Universe);
    fn track(&mut self, universe: &Universe, deleted: &[usize], added: &[usize]);
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

    pub fn dirty(&mut self) -> &mut Self {
        self.need_flush = true;
        self
    }
}
