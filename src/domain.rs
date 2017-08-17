#![macro_use]
use std::fmt;
use std::collections::HashMap;
use std::sync::{RwLock, Arc};

use intern;
use intern::PBox;
use property::{GlobalPropertyId, PropertyName, DomainedPropertyId};

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy, PartialOrd, Ord)]
/**
 * Each property name is in the form "domain/property_name".
 * This struct represents the `domain` part of the property name, and is also used to reference
 * that domain by calling `domain_name.register()`.
 * */
pub struct DomainName(pub &'static str);
impl fmt::Display for DomainName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
        // requires a lock: write!(f, "{:?} ({:?})", self.0, self.get_id())
    }
}
impl DomainName {
    pub fn register(&self) {
        intern::check_name(self.0);
        let globals = clone_globals();
        let mut properties = globals.write().unwrap();
        let next_did = DomainId(properties.domains.len());
        {
            use std::collections::hash_map::Entry;
            let entry = properties.domains.entry(*self);
            if let Entry::Occupied(entry) = entry {
                assert_eq!(&entry.get().name, self);
                return;
            }
            entry.or_insert_with(|| {
                DomainInfo {
                    id: next_did,
                    name: *self,
                    property_members: Vec::new(),
                    tables: HashMap::new(),
                    locked: false,
                    name2did: HashMap::new(),
                }
            });
        }
        properties.did2name.push(*self);
        debug_assert_eq!(&properties.did2name[next_did.0], self);
    }

    fn map_info<R, F: Fn(&DomainInfo) -> R>(&self, f: F) -> R {
        let globals = clone_globals();
        let properties = globals.read().unwrap();
        let info = properties.domains.get(self).unwrap_or_else(|| panic!("{:?} is not a registered domain", self));
        f(info)
    }

    fn map_info_mut<R, F: Fn(&mut DomainInfo) -> R>(&self, f: F) -> R {
        let globals = clone_globals();
        let mut properties = globals.write().unwrap();
        let info = properties.domains.get_mut(self).unwrap_or_else(|| panic!("{:?} is not a registered domain", self));
        f(info)
    }

    pub fn get_id(&self) -> DomainId {
        self.map_info(|info| info.id)
    }

    pub fn locked(&self) -> bool {
        self.map_info(|info| info.locked())
    }

    pub fn set_locked(&self, is_locked: bool) {
        self.map_info_mut(|info| info.set_locked(is_locked))
    }
}

#[inline]
pub fn check_lock() -> bool {
    !cfg!(test)
}

/**
 * Declares a domain. This is equivalent to a namespace, but only has one level.
 * 
 * Domains are used in `property!`s and `table!`s.
 * 
 * # Usage
 *
 * ```
 * # #[macro_use] extern crate v11;
 * domain! { DOMAIN_NAME }
 * // or domain! { pub DOMAIN_NAME }
 *
 * fn main() {
 *     DOMAIN_NAME.register();
 * }
 * ```
 *
 * A 'true name' can be used to disambiguate same-named domains in different libraries.
 * ```
 * # #[macro_use] extern crate v11;
 * domain! { DOMAIN_NAME ("TRUE_NAME") }
 * ```
 *
 * */
#[macro_export]
macro_rules! domain {
    (pub $name:ident) => {
        domain! { pub $name (stringify!($name)) }
    };
    ($name:ident) => {
        domain! { $name (stringify!($name)) }
    };
    (pub $name:ident ($truename:expr)) => {
        pub const $name: $crate::domain::DomainName = $crate::domain::DomainName($truename);
    };
    ($name:ident ($truename:expr)) => {
        const $name: $crate::domain::DomainName = $crate::domain::DomainName($truename);
    };
}

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub struct DomainId(pub usize);

use tables::{TableName, GenericTable};

pub struct DomainInfo {
    pub id: DomainId,
    pub name: DomainName,
    pub property_members: Vec<GlobalPropertyId>,
    pub tables: HashMap<TableName, GenericTable>,
    pub name2did: HashMap<PropertyName, DomainedPropertyId>,
    locked: bool,
}
impl fmt::Debug for DomainInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "\t\t\tDomainInfo: {:?}", self.name)?;
        let globals = clone_globals();
        match globals.try_read() {
            Ok(pmap) => {
                for m in &self.property_members {
                    writeln!(f, "\t\t\t\t{:?}: {:?}", m, pmap.gid2name.get(&m))?;
                }
            },
            _ => {
                writeln!(f, "\t\t\t\t<Lock inaccessible>")?;
            },
        }
        write!(f, "")
    }
}
impl DomainInfo {
    pub fn instantiate(&self, gid2producer: &[fn() -> PBox]) -> DomainInstance {
        if check_lock() && !self.locked() { panic!("not locked"); }
        let properties = self.property_members.iter().map(|id| {
            gid2producer[id.0]()
        }).collect();
        let tables = self.tables.iter().map(|(k, v)| (*k, v.prototype().guard())).collect();
        DomainInstance {
            id: self.id,
            name: self.name,
            property_members: properties,
            tables: tables,
        }
    }

    pub fn locked(&self) -> bool { self.locked }

    pub fn set_locked(&mut self, locked: bool) {
        self.locked = locked
    }
}

#[derive(Debug)]
pub enum MaybeDomain {
    /// The Universe does not have this domain.
    Unset(DomainName),
    /// The Universe does have that domain.
    Domain(DomainInstance),
}
impl MaybeDomain {
    fn is_set(&self) -> bool {
        match self {
            &MaybeDomain::Unset(_) => false,
            &MaybeDomain::Domain(_) => true,
        }
    }
}

use Universe;
impl Universe {
    pub fn get_domains(domains: &[DomainName]) -> Vec<MaybeDomain> {
        let globals = clone_globals();
        let mut pmap = &mut *globals.write().unwrap();
        let mut ret = (0..pmap.domains.len()).map(|x| MaybeDomain::Unset(pmap.did2name[x])).collect::<Vec<MaybeDomain>>();
        for name in domains.iter() {
            let did = pmap.domains.get(name).unwrap_or_else(|| {
                panic!("Unregistered domain {}", name)
            }).id.0;
            ret[did] = pmap.instantiate_domain(*name);
        }
        ret
    }

    pub fn add_domain(&mut self, domain: DomainName) {
        self.sync_domain_list();
        let id = domain.get_id().0;
        if self.domains[id].is_set() { return; }
        let globals = clone_globals();
        self.domains[id] = globals.write().unwrap().instantiate_domain(domain);
    }

    /// Make sure this Universe has a MaybeDomain for every globally registered DomainName.
    fn sync_domain_list(&mut self) {
        let globals = clone_globals();
        let properties = globals.read().unwrap();
        let news = &properties.did2name[self.domains.len()..];
        self.domains.extend(news.iter().map(|d| MaybeDomain::Unset(*d)));
    }

    /// Adds any properties that are unknown. This function should be called if any libraries have
    /// been loaded since before the universe was created.
    pub fn add_properties(&mut self) {
        // We only allow domains to be set at creation, so we don't need to look for new ones.
        // Trying to get a property at a new domain is an errorneous/exceptional case, so this is
        // fine.
        let globals = clone_globals();
        let pmap = globals.read().unwrap();
        for prop in &mut self.domains {
            if let MaybeDomain::Domain(ref mut instance) = *prop {
                instance.add_properties(&*pmap);
            }
        }
    }

    /// Return a list of the names of all registered domains.
    pub fn get_domain_names(&self) -> Vec<DomainName> {
        let mut ret = Vec::new();
        for domain in &self.domains {
            if let MaybeDomain::Domain(ref instance) = *domain {
                ret.push(instance.name);
            }
        }
        ret
    }
}

pub struct DomainInstance {
    pub id: DomainId,
    pub name: DomainName,
    pub property_members: Vec<PBox>,
    // FIXME: Tables can have domained_index as well, so we can ditch the HashMap for O(1).
    pub tables: HashMap<TableName, RwLock<GenericTable>>,
}
impl fmt::Debug for DomainInstance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let globals = clone_globals();
        let pmap = globals.read().unwrap();
        writeln!(f, "id: {:?}, name: {:?}, property_members.len(): {:?}", self.id, self.name, self.property_members.len())?;
        writeln!(f, "properties: {:?}", pmap.domains[&self.name])?;
        write!(f, "tables:")?;
        for n in self.tables.keys() {
            write!(f, " {:?}", n.0)?;
        }
        Ok(())
    }
}
impl DomainInstance {
    pub fn add_properties(&mut self, all_properties: &GlobalProperties) {
        let info = &all_properties.domains[&self.name];
        while self.property_members.len() < info.property_members.len() {
            let gid = info.property_members[self.property_members.len()];
            let producer = all_properties.gid2producer[gid.0];
            self.property_members.push(producer());
        }
    }

    pub fn get_generic_table(&self, name: TableName) -> &RwLock<GenericTable> {
        self.tables.get(&name).unwrap_or_else(|| {
            println!("Tables in this domain:");
            for t in self.tables.keys() {
                println!("{}", t);
            }
            panic!("Table {:?} is not in domain {:?}", name, self.name);
        })
    }
}

#[derive(Default)]
#[derive(Debug)]
// FIXME: Rename to...? Multiverse?
pub struct GlobalProperties {
    // PropertyName is stringly domained; no worry about collisions.
    pub name2gid: HashMap<PropertyName, GlobalPropertyId>,
    pub gid2name: HashMap<GlobalPropertyId, PropertyName>,

    pub gid2producer: Vec<fn() -> PBox>,
    pub domains: HashMap<DomainName, DomainInfo>,
    pub did2name: Vec<DomainName>,
}
impl GlobalProperties {
    fn instantiate_domain(&mut self, domain: DomainName) -> MaybeDomain {
        {
            match self.domains.get_mut(&domain) {
                None => panic!("Unregistered domain {}", domain),
                Some(d) => d.set_locked(true),
            }
        }
        let domain_info = &self.domains[&domain];
        MaybeDomain::Domain(domain_info.instantiate(&self.gid2producer))
    }

    fn is_empty(&self) -> bool {
        self.domains.is_empty() && self.gid2producer.is_empty()
    }

    fn id(&self) -> usize {
        self as *const Self as usize
    }
}

pub type GlobalsLock = Arc<RwLock<GlobalProperties>>;

lazy_static! {
    /// Stupidly complicated. It is difficult to guarantee that dynamic libraries do or do not link
    /// this global. So `main` should call `clone_globals` and pass it into each dynamic library, which
    /// will then need to call `sync_globals`.
    ///
    /// RwLock: We need to be able to modify the Arc.
    /// Arc: We want to share with other modules.
    /// Inner lock: actual thing that is mutated
    pub static ref V11_GLOBALS: RwLock<GlobalsLock> = Default::default();
}

pub fn clone_globals() -> GlobalsLock {
    V11_GLOBALS.read().unwrap().clone()
}

pub fn sync_globals(globals: GlobalsLock) {
    let mut old = V11_GLOBALS.write().unwrap();
    {
        let old = old.read().unwrap();
        if old.id() == globals.read().unwrap().id() {
            // Duplicate call, calling on self, or libraries linked.
            return;
        }
        if !old.is_empty() {
            panic!("sync_globals overriding globals that have already been set")
        }
    }
    *old = globals;
}


#[cfg(test)]
mod tests {
    #[test]
    fn register_domains_once() {
        domain! { A }
        domain! { B }
        A.register();
        B.register();
    }

    #[test]
    fn register_domain_multiple_times() {
        domain! { MULTI_REG }
        domain! { SINGLE }
        MULTI_REG.register();
        MULTI_REG.register();
        SINGLE.register();
    }
}
