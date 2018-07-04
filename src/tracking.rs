use tables::TableRow;

pub type EventHandlerInput<'a> = &'a mut Iterator<Item=usize>;
pub type FallbackEventHandler = fn(&Universe, &mut GenericTable, Event, EventHandlerInput);


/// Does nothing.
pub fn null_event_handler(_: &Universe, _: &mut GenericTable, _: Event, _: EventHandlerInput) {}
pub(crate) fn invalid_event_handler(_: &Universe, _: &mut GenericTable, _: Event, _: EventHandlerInput) { panic!("invalid event"); }

/// The indicated rows are deleted.
pub fn deletion_event_handler(universe: &Universe, table: &mut GenericTable, event: Event, rows: EventHandlerInput) {
    table.dyn_table.remove_rows(universe, event, rows);
    let flush = table.acquire_flush();
    flush.flush(universe, event::INVALID_EVENT, event);
}

impl Universe {
    /// Register a `FallbackEventHandler` for the given `Event`. The old handler is returned.
    pub fn add_generic_event_handler(&self, event: Event, new: FallbackEventHandler) -> FallbackEventHandler {
        if event == event::INVALID_EVENT {
            panic!("Can't change INVALID_EVENT");
        }
        if event.0 > MAX_EVENTS {
            panic!("Excessively large event number.");
        }
        let old = self.generic_event_handler(event);
        self.event_handlers[event.0] = new;
        while self.event_handlers.len() < event.0 {
            self.event_handlers.push(null_event_handler);
        }
        self.event_handlers.push(new);
        let new2 = self.generic_event_handler(event);
        assert_eq!(new2 as usize, new as usize);
        old
    }

    /// Return the FallbackEventHandler` for the given `Event`. If there is no registered handler,
    /// then `null_event_handler` is returned.
    pub fn generic_event_handler(&self, event: Event) -> FallbackEventHandler {
        self.event_handlers.get(event.0)
            .map(|r| *r)
            .unwrap_or(null_event_handler as FallbackEventHandler)
    }
}

const MAX_EVENTS: usize = 512;
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Event(usize);

/// An arbitrary collection of verbs for you to use for table events.
/// The semantics of these are user-defined.
/// If you need another name, submit a PR, or create an `Other(u8)` const.
pub mod event {
    macro_rules! events {
        ($($ident:ident = $n:expr),*) => {
            use super::Event;
            $(
                pub const $ident: Event = Event($n);
            )*
        }
    }
    events! {
        INVALID_EVENT = 0,
        CREATE = 1,
        DELETE = 2,
        SAVE = 3,
        SYNC = 4,
        UNSYNC = 5,
        UNLOAD = 6,
        UPDATE = 7,
        MODIFY = 8,
        MOVE = 9,
        DIRTY = 10,
        RESET = 11,
        VIEW = 12,
        DEBUG = 13,
        CLONE = 14
    }

}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum Dependency {
    Handle,
    Ignore,
    Delegate,
}


/// `Tracker`s are notified of structural changes to tables. This requires the 'consistent'
/// guarantee, which is provided by `#[kind = "public"]`.
// FIXME: https://github.com/rust-lang/rust/issues/29628
// https://doc.rust-lang.org/beta/unstable-book/language-features/on-unimplemented.html
// #[rustc_on_unimplemented = "You must implement `Tracker` on `{Self}` so that it can react
// to structural changes in the `#[foreign]` table."]
pub trait Tracker: Send + Sync + 'static {
    type Foreign: TableRow;

    /// Indicate this tracker's interest in the event.
    /// Unknown events should probably be handled by returning `Dependency::Delegate`.
    fn dependency(&self, event: Event) -> Dependency;

    /// The foreign table was cleared. Perhaps clearing the local table would be appropriate?
    fn cleared(&self, universe: &Universe, event: Event);

    /// Handle the events.
    /// The indicated foreign rows have been added, or deleted, or whatever.
    /// Modifications to the local table should be made as appropriate to maintain consistency.
    ///
    /// Very often you will maintain consistency by deleting rows whose foreign key has been
    /// deleted.
    /// If the foreign key's column has an `#[index]`,
    /// you can call`$table.track_$col_events(deleted)`.
    /// Or, if the table is `#[kind = "sorted"]` and has a `#[sort_key]` column,
    /// you can call `$table.track_sorted_$col_events(deleted)`.
    ///
    /// You may lock the foreign table for editing,
    /// but making structural changes will almost certainly cause you trouble!
    ///
    /// If the foreign key's column has `#[sort_key]`, then the events are sorted.
    /// Otherwise, the order is undefined.
    // (Well, the order is the order the events occured in unless one of the trackers needs them to
    // be sorted.)
    ///
    /// Any deleted rows in the foreign table will still be valid (at least so far as you'll be
    /// able to access their contents without error), but will become actually-deleted after the
    /// flush completes.
    ///
    /// Any newly created rows have just become accessible.
    fn selected(
        &self,
        universe: &Universe,
        event: Event,
        foreign: &mut Iterator<Item=Self::Foreign>,
    );

    // FIXME: usize instead of GenericRowId.
    // This'd break object safety tho... GenericTable has:
    //  - deleted: Vec<usize>, 
    //  - Vec<Box<Tracker>>
    // Might need to box a tracker container trait.
    // FIXME: Maybe separate 'track_delete' and 'track_add' fns? What about all three?
}

#[doc(hidden)]
#[derive(Default)]
pub struct TrackInfo {
    pub del: Vec<usize>,
    pub add: Vec<usize>,
    pub table_cleared: bool,
    pub has_trackers: bool,
}
impl TrackInfo {
    pub fn prototype(&self) -> Self {
        TrackInfo {
            has_trackers: self.has_trackers,
            // We don't clone because there might be residual events.
            .. Default::default()
        }
    }
}

use std::sync::{Arc, RwLock};
use std::mem;
use tables::GenericTable;
use Universe;
use intern::PBox;

struct SomeTracker {
    inner: PBox, // Box<Tracker<Foreign=???> + Send + Sync>,
    sort: bool,
}
impl SomeTracker {
    fn cast<F>(&self) -> &Tracker<Foreign=F> {
        self.inner.downcast_ref().unwrap()
    }
}

#[doc(hidden)]
#[must_use]
pub struct GenericFlush {
    trackers: Arc<RwLock<Vec<SomeTracker>>>,
    tracking_info: TrackInfo,
}
#[doc(hidden)]
impl GenericFlush {
    pub fn flush<F: TableRow>(&mut self, universe: &Universe, ev_add: Event, ev_del: Event) {
        // FIXME: Someone could get at the table before we finish flushing!
        // We might need a single Universe.mutex or something?
        // But that sort of lock-interweaving is very difficult I hear...
        // Like C++ has some special lock-management algorithm?
        #[derive(Default)]
        struct EvReq {
            need: bool,
            fallback: FallbackEventHandler,
        }
        impl EvReq {
            fn eval<F: TableRow>(universe: &Universe, event: Event, trackers: &[SomeTracker], rows: &mut [usize]) -> EvReq {
                let fallback = universe.generic_event_handler(event);
                let mut ret = EvReq { need: false, fallback };
                if rows.is_empty() { return ret; }
                let mut sort = false;
                for tracker in trackers {
                    if tracker.cast::<F>().dependency(event) == Dependency::Ignore { continue; }
                    ret.need = true;
                    sort |= tracker.sort
                }
                if sort {
                    rows.sort();
                }
                ret
            }
            fn pass<F: TableRow>(&self, universe: &Universe, tracker: &Tracker<Foreign=F>, event: Event, rows: &[usize]) {
                if !self.need { return; }
                match tracker.dependency(event) {
                    Dependency::Ignore => (),
                    Dependency::Handle => tracker.selected(universe, event, rows),
                    Dependency::Delegate => self.fallback(universe, event, rows),
                }
            }
        }
        let trackers = self.trackers.read().unwrap();
        let trackers = &trackers[..];
        let add = EvReq::eval(universe, ev_add, trackers, &mut self.tracking_info.add[..]);
        let del = EvReq::eval(universe, ev_del, trackers, &mut self.tracking_info.del[..]);
        for tracker in trackers {
            let tracker = tracker.cast();
            if self.tracking_info.table_cleared {
                tracker.cleared(universe, ev_del);
            }
            del.pass::<F>(universe, tracker, ev_add, &self.tracking_info.del[..]);
            add.pass::<F>(universe, tracker, ev_del, &self.tracking_info.add[..]);
        }
        self.tracking_info.table_cleared = false;
        self.tracking_info.del.clear();
        self.tracking_info.add.clear();
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
    pub fn need_flush(&self) -> bool {
        !self.info.delete.is_empty() || !self.info.add.is_empty() || self.info.cleared
    }

    pub fn unflushed_summary(&self) -> String {
        format!("delete: {}, add: {}, cleared: {}",
                self.info.delete.len(), self.info.add.len(), self.info.cleared)
    }

    pub fn acquire_flush(&mut self) -> GenericFlush {
        let mut tracking_info = TrackInfo::default();
        mem::swap(&mut tracking_info, &mut self.tracking_info);
        GenericFlush {
            trackers: self.trackers.clone(),
            tracking_info,
        }
    }

    pub fn cleared(&mut self) {
        self.tracking_info.cleared = true;
        self.tracking_info.add.clear();
        self.tracking_info.delete.clear();
        self.free.clear();
    }

    pub fn add_tracker(&mut self, inner: PBox, sort_events: bool) {
        if !self.guarantee.consistent {
            panic!("Tried to add tracker to inconsistent table, {}/{}", self.domain, self.name);
        }
        self.trackers.write().unwrap().push(SomeTracker { inner, sort_events });
        self.tracking_info.has_trackers = true;
    }

    fn skip_events(&self) -> bool { !self.has_trackers }

    pub fn delete(&mut self, i: usize) {
        self.free.insert(i, ()); // freelist needs to stay up to date even if nobody's watching
        if self.skip_events() { return; }
        self.delete.push(i);
    }

    pub fn add(&mut self, i: usize) {
        if self.skip_events() { return; }
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
