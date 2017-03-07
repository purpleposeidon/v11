#![macro_use]
use std::fmt;
use std::collections::HashMap;
use std::sync::RwLock;

use intern;
use intern::PBox;
use property::{GlobalPropertyId, PropertyName};

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
/**
 * Each property name is in the form "domain/property_name".
 * This struct represents the `domain` part of the property name, and is also used to reference
 * that domain by calling `domain_name.register()`.
 * */
pub struct DomainName(pub &'static str);
impl fmt::Display for DomainName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}
impl DomainName {
    pub fn register_domain(&self) {
        intern::check_name(self.0);
        let mut properties = PROPERTIES.write().unwrap();
        let next_id = DomainId(properties.domains.len());
        properties.domains.entry(*self).or_insert_with(|| {
            DomainInfo {
                id: next_id,
                name: *self,
                property_members: Vec::new(),
            }
        });
        properties.did2name.push(*self);
        debug_assert_eq!(&properties.did2name[next_id.0], self);
    }
}

#[macro_export]
macro_rules! domain {
    (pub $name:ident) => {
        pub const $name: $crate::domain::DomainName = $crate::domain::DomainName(stringify!($name));
    };
    ($name:ident) => {
        const $name: $crate::domain::DomainName = $crate::domain::DomainName(stringify!($name));
    };
}

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub struct DomainId(pub usize);


pub struct DomainInfo {
    pub id: DomainId,
    pub name: DomainName,
    pub property_members: Vec<GlobalPropertyId>,
}
impl fmt::Debug for DomainInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let pmap = PROPERTIES.read().unwrap();
        writeln!(f, "\t\t\tDomainInfo: {:?}", self.name)?;
        for m in &self.property_members {
            writeln!(f, "\t\t\t\t{:?}: {:?}", m, pmap.gid2name.get(&m))?;
        }
        write!(f, "")
    }
}
impl DomainInfo {
    pub fn instantiate(&self, gid2producer: &[fn() -> PBox]) -> DomainInstance {
        DomainInstance {
            id: self.id,
            name: self.name,
            property_members: self.property_members.iter().map(|id| {
                gid2producer[id.0]()
            }).collect(),
        }
    }
}

#[derive(Debug)]
pub enum MaybeDomain {
    /// The Universe does not have this domain.
    Unset(DomainName),
    /// The Universe does have that domain.
    Domain(DomainInstance),
}

pub struct DomainInstance {
    pub id: DomainId,
    pub name: DomainName,
    pub property_members: Vec<PBox>,
}
impl fmt::Debug for DomainInstance {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let pmap = PROPERTIES.read().unwrap();
        writeln!(f, "id: {:?}, name: {:?}, property_members.len(): {:?}", self.id, self.name, self.property_members.len())?;
        writeln!(f, "{:?}", pmap.domains[&self.name])
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
}

#[derive(Default)]
#[derive(Debug)]
pub struct GlobalProperties {
    pub name2gid: HashMap<PropertyName, GlobalPropertyId>,
    pub gid2name: HashMap<GlobalPropertyId, PropertyName>,

    pub gid2producer: Vec<fn() -> PBox>,
    pub domains: HashMap<DomainName, DomainInfo>,
    pub did2name: Vec<DomainName>,
}

lazy_static! {
    pub static ref PROPERTIES: RwLock<GlobalProperties> = Default::default();
}

