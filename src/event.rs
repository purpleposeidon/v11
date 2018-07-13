//! An arbitrary collection of verbs for you to use for table events.
//! The precise meaning of the names of the events is user-defined.
//!
//! If you need another name, submit a PR, or create a custom `const Event`.


#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Event {
    /// A unique ID number. Used as an index for fallback handlers.
    pub id: u16,
    pub is_removal: bool,
    pub is_creation: bool,
}

const Z: u8 = 0;
const C: u8 = 1;
const D: u8 = 2;

macro_rules! events {
    ($($(#[$attr:meta])* $mode:ident:$ident:ident = $id:expr,)*) => {
        $(
            $(#[$attr])*
            pub const $ident: Event = Event {
                id: $id,
                is_removal: $mode & D > 0,
                is_creation: $mode & C > 0,
            };
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
events! {
    Z:INVALID_EVENT = 0,

    C:CREATE = 1,
    D:DELETE = 2,

    Z:SERIALIZE = 3,
    C:DESERIALIZE = 4,

    Z:SAVE = 5,
    D:UNLOAD = 6,

    C:SYNCED = 7,
    D:UNSYNCED = 8,

    C:MOVE_IN = 9,
    D:MOVE_OUT = 10,

    Z:UPDATE = 11,
    Z:MODIFY = 12,
    Z:DIRTY = 13,
    Z:RESET = 14,

    Z:VIEW = 15,
    Z:DEBUG = 16,
    Z:CLONE = 17,
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
                .map(|_| Box::new(NullHandler) as Box<FallbackHandler>)
                .collect(),
        }
    }
}
impl EventHandlers {
    pub fn add(&mut self, _event: Event, _handler: Box<FallbackHandler>) -> Box<FallbackHandler> {
        unimplemented!("converting a list of foreign rows to local rows doesn't have a trivial implementation");
        /*if event == INVALID_EVENT {
            panic!("Can't set the INVALID_EVENT handler");
        }
        ::std::mem::swap(
            &mut self.fallbacks[event.id as usize],
            &mut handler,
        );
        handler*/
    }

    /// Return the FallbackHandler` for the given `Event`. If there is no registered handler,
    /// then `null_event_handler` is returned.
    pub fn get(&self, event: Event) -> &FallbackHandler {
        self.fallbacks[event.id as usize].as_ref()
    }
}
