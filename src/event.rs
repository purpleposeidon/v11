//! An arbitrary collection of verbs for you to use for table events.
//! The precise meaning of the names of the events is user-defined.
//!
//! If you need another name, submit a PR, or create a custom `const Event`.


#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Event {
    /// A unique ID number. Used as an index for fallback handlers.
    pub id: u16,
    /// If `true`, then auto-derived foreign rows will remove the selected rows.
    pub is_removal: bool,
    /// The disposition of auto-derived foreign rows.
    pub delegate: Disposition,
}

macro_rules! events {
    ($($(#[$attr:meta])* $ident:ident = $id:expr, $rm:expr, $del:expr,)*) => {
        $(
            $(#[$attr])*
            pub const $ident: Event = Event { id: $id, is_removal: $rm, delegate: $del };
        )*
        use std::fmt;
        impl fmt::Debug for Event {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let s = match self.id {
                    $($id => stringify!($ident),)*
                    _ => return write!(f, "CUSTOM_{}", self.id),
                };
                write!(f, "{}", s)
            }
        }
        pub static EVENT_LIST: &[Event] = &[$($ident),*];
    }
}
use self::Disposition::*;
events! {
    INVALID_EVENT = 0, false, Delegate,
    CREATE = 1, false, Ignore,
    DELETE = 2, true, Handle,
    SAVE = 3, false, Delegate,
    SYNC = 4, false, Delegate,
    UNSYNC = 5, true, Handle,
    UNLOAD = 6, true, Handle,
    UPDATE = 7, false, Ignore,
    MODIFY = 8, false, Ignore,
    MOVE = 9, false, Ignore,
    DIRTY = 10, false, Ignore,
    RESET = 11, false, Delegate,
    VIEW = 12, false, Delegate,
    DEBUG = 13, false, Delegate,
    CLONE = 14, false, Delegate,
}

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum Disposition {
    /// The `Tracker` will handle the event.
    Handle,
    /// Have the `Universe`'s default handler deal with the event.
    Delegate,
    /// The `Tracker` will handle the event, but then again delegate it.
    Inspect,
    /// Ignore the event. This is more efficient than returning `Handle` and ignoring.
    Ignore,
}

// Fallback event handlers must call methods on GenericTable, and its parameters must be cast
// through &Any.

use Universe;
use tables::GenericTable;
use tracking::SelectAny;


pub trait FallbackHandler: 'static + Send + Sync {
    fn needs_sort(&self, gt: &GenericTable) -> bool;
    fn handle(&self, universe: &Universe, gt: &mut GenericTable, event: Event, rows: SelectAny);
}

pub struct NullHandler;
impl FallbackHandler for NullHandler {
    fn needs_sort(&self, _gt: &GenericTable) -> bool { false }
    fn handle(&self, _universe: &Universe, _gt: &mut GenericTable, _event: Event, _rows: SelectAny) {}
}

pub struct InvalidHandler;
impl FallbackHandler for InvalidHandler {
    fn needs_sort(&self, _gt: &GenericTable) -> bool { false }
    fn handle(&self, _universe: &Universe, _gt: &mut GenericTable, _event: Event, _rows: SelectAny) { panic!("Invalid fallback event handler!"); }
}

pub struct DeleteHandler;
impl FallbackHandler for DeleteHandler {
    fn needs_sort(&self, gt: &GenericTable) -> bool {
        gt.guarantee.sorted
    }
    fn handle(&self, universe: &Universe, gt: &mut GenericTable, event: Event, rows: SelectAny) {
        gt.table.remove_rows(universe, event, rows);
    }
}

/// Maximum allowed
pub const MAX_EVENT_TYPES: usize = 32;
pub struct EventHandlers {
    fallbacks: Vec<Box<FallbackHandler>>,
}
impl Default for EventHandlers {
    fn default() -> Self {
        EventHandlers {
            fallbacks: (0..MAX_EVENT_TYPES)
                .map(|i| {
                    if EVENT_LIST.get(i).map(|e| e.delegate) == Some(Ignore) {
                        Box::new(NullHandler) as Box<FallbackHandler>
                    } else {
                        Box::new(InvalidHandler) as Box<FallbackHandler>
                    }
                }).collect(),
        }
    }
}
impl EventHandlers {
    pub fn add(&mut self, event: Event, mut handler: Box<FallbackHandler>) -> Box<FallbackHandler> {
        if event == INVALID_EVENT {
            panic!("Can't set the INVALID_EVENT handler");
        }
        ::std::mem::swap(
            &mut self.fallbacks[event.id as usize],
            &mut handler,
        );
        handler
    }

    /// Return the FallbackHandler` for the given `Event`. If there is no registered handler,
    /// then `null_event_handler` is returned.
    pub fn get(&self, event: Event) -> &FallbackHandler {
        self.fallbacks[event.id as usize].as_ref()
    }
}
