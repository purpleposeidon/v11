//! A column-based in-memory database for [Data-Oriented Design][dod].
//! 
//! # Safety
//! This crate *lies about safety*.
//! The intention is that nothing goes wrong, so long as 
//! 
//! This is no problem so long as
//! the registration of properties, tables, and domains is cleanly separated from their usage.
//! 
//! [dod]: (http://www.dataorienteddesign.com/dodmain/)

#[allow(unused_imports)]
#[macro_use]
// We don't actually use macros or the derive, but this silences up a warning.
extern crate v11_macros;
#[macro_use]
extern crate procedural_masquerade;
extern crate rustc_serialize;
extern crate itertools;
extern crate bit_vec_mut as bit_vec;
extern crate num_traits;
#[macro_use]
extern crate lazy_static;

use std::sync::*;


pub mod domain;
pub mod tables;
pub mod property;
pub mod intern;
pub mod columns;
pub mod index;
pub mod storage;
pub mod tracking;

pub mod joincore;
pub mod context;
mod assert_sorted;


/**
 * Trait that all storable types must implement.
 *
 * Types that implement this trait should also not implement `Drop`, although this is not yet
 * expressable, and is not presently required.
 * */
pub trait Storable: Sync + Sized /* + !Drop */ {}
impl<T> Storable for T where T: Sync + Sized /* + !Drop */ {}


pub type GuardedUniverse = Arc<RwLock<Universe>>;

pub use domain::DomainName;
use domain::MaybeDomain;

/**
 * A context object whose reference should be passed around everywhere.
 * */
pub struct Universe {
    pub domains: Vec<MaybeDomain>,
}
impl Universe {
    // FIXME: Mark this unsafe?
    pub fn new(domains: &[DomainName]) -> Universe {
        let mut ret = Universe {
            domains: Self::get_domains(domains),
        };
        for domain in domains {
            ret.init_domain(*domain);
        }
        ret
    }

    pub fn guard(self) -> GuardedUniverse { Arc::new(RwLock::new(self)) }

    /**
     * Returns a string describing all the tables in the Universe.
     * (This does not include their contents.)
     * */
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

