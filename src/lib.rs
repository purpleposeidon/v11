//! A column-based in-memory database for [Data-Oriented Design][dod].
//!
//! # Safety
//! This crate *lies about safety*. Rust wants to be monkey-safe; v11 aims for merely derp-safe.
//!
//! The danger zones are easy to avoid:
//!
//! - Calling `register()` functions on domains, properties and tables. This should only be done from the
//! main thread, before any `Universe`s have been created.
//! - Doing strange things with the references in a table lock. (Namely, `mem::swap`.)
//! - Using `pub` items that are marked `#[doc(hidden)]`. These should only be used by
//! macro-generated code.
//!
//!
//! [dod]: http://www.dataorienteddesign.com/dodmain/
// FIXME: Change `References` to [`References`].
// FIXME: Could use some hefty reorganization.

#[allow(unused_imports)]
#[macro_use]
// We don't actually use macros or the derive, but this silences a warning.
extern crate v11_macros;
#[macro_use]
extern crate procedural_masquerade;
pub extern crate serde;
#[macro_use]
extern crate serde_derive;
pub extern crate erased_serde; // There are many things that need to be erased.
extern crate itertools;
#[doc(hidden)]
pub extern crate num_traits;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate mopa;

use std::sync::*;


#[macro_use]
pub mod domain;
#[macro_use]
pub mod table_macro;
pub mod tables;
#[macro_use]
pub mod property;
#[doc(hidden)]
pub mod intern;
pub mod columns;
pub mod index;
pub mod map_index;
pub mod storage;
pub mod tracking;
pub mod event;

#[macro_use]
pub mod context;

// Util. Buncha these could become crates!
pub mod joincore;
mod assert_sorted;
pub mod any_slice;
mod serial;

// FIXME: #[cfg(rustdoc)] will be a thing eventually.
#[cfg(feature = "doc")]
pub mod examples;

#[cfg(not(feature = "doc"))]
/// Run `cargo doc --features doc --open` to get documentation on example macro output.
pub mod examples {}

#[cfg(feature = "doc")]
#[doc(hidden)]
pub mod v11 {
    pub use super::*;
    // This is a work around for not having $crate in the procedural_masquerade.
    // It is necessary for the macro invokations in `examples`.
    // It might be possible to emulate $crate...
}


/// Trait describing bounds that all storable types must satisfy.
///
/// Types that implement this trait shouldn't implement `Drop`,
/// and they shouldn't be `mem::needs_drop`.
/// However this can not yet be expressed... and actually isn't even required yet.
///
/// There are additional requirements not expressed by this type.
// FIXME: !Drop
pub trait Storable: 'static + Send + Sync + Sized /* + !Drop */ {
    #[doc(hidden)]
    fn assert_no_drop() {
        // FIXME: Call me, maybe.
        if ::std::mem::needs_drop::<Self>() {
            panic!("Column element needs_drop");
        }
    }
}

impl<T> Storable for T where T: 'static + Send + Sync + Sized /* + !Drop */ {}

pub type GuardedUniverse = Arc<RwLock<Universe>>;

use crate::domain::{DomainName, MaybeDomain};

/**
 * A context object whose reference should be passed around everywhere.
 * */
pub struct Universe {
    #[doc(hidden)] pub domains: Vec<MaybeDomain>,
    pub event_handlers: crate::event::EventHandlers,
}

/// Universe manipulation methods.
impl Universe {
    // FIXME: Mark this unsafe?
    pub fn new(domains: &[DomainName]) -> Universe {
        let mut ret = Universe {
            domains: Self::get_domains(domains),
            event_handlers: Default::default(),
        };
        for domain in domains {
            ret.init_domain(*domain);
        }
        ret
    }

    /// Converts to a form shareable with other threads.
    pub fn guard(self) -> GuardedUniverse { Arc::new(RwLock::new(self)) }

    /// Returns a string describing all the tables in the Universe.
    /// (This does not include their contents.)
    pub fn info(&self) -> String {
        let mut out = "".to_owned();
        for domain in &self.domains {
            let domain = match *domain {
                MaybeDomain::Unset(_) => continue,
                MaybeDomain::Domain(ref i) => i,
            };
            use itertools::Itertools;
            let info: String = domain.tables.iter().map(|(_, table)| {
                table.read().unwrap().info()
            }).join(" ");
            out += &format!("{}: {}\n", domain.name, info);
        }
        out
    }
}
use std::fmt;
impl fmt::Debug for Universe {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Universe:")?;
        for domain in &self.domains {
            writeln!(f, "\t{:?}", domain)?;
        }
        write!(f, "")
    }
}

/// Return value for function parameters passed into `$table.visit`.
#[derive(Debug, Clone)]
pub enum Action<IT> {
    /// If `remove` is true, then this row is removed.
    /// Any items yielded by `add` are inserted prior to the next item.
    /// They will not be visited.
    Continue {
        remove: bool,
        add: IT,
    },
    /// Stop visiting rows. They will not be removed.
    Break,
}
