
/// `Tracker`s are notified of structural changes to tables. This requires the 'consistent'
/// guarantee, which is provided by `#[kind = "public"]`.
pub trait Tracker {
    fn track(&self, deleted: &[usize], added: &[usize]);
    fn cleared(&self);
}

use std::sync::{Arc, RwLock};
use std::mem;
use tables::GenericTable;

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
    pub fn flush(&mut self) {
        let mut trackers = self.trackers.write().unwrap();
        for tracker in trackers.iter_mut() {
            if self.cleared {
                tracker.cleared();
            }
            tracker.track(&self.delete[..], &self.add[..]);
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
}
