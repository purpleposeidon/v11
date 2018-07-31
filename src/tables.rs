use std::any::Any;
use std::sync::*;
use std::fmt;


pub use v11_macros::*;

use Universe;
use intern;
use intern::PBox;
use domain::{DomainName, DomainId, MaybeDomain};

impl Universe {
    #[doc(hidden)]
    pub fn get_generic_table(&self, domain_id: DomainId, name: TableName) -> &RwLock<GenericTable> {
        if let Some(&MaybeDomain::Domain(ref domain)) = self.domains.get(domain_id.0) {
            return domain.get_generic_table(name);
        }
        panic!("Request for table {} in unknown domain #{}", name, domain_id.0);
    }
}

type Prototyper = fn() -> PBox;


use tracking;
pub trait TTable: ::mopa::Any + Send + Sync {
    // A slightly annoying thing is that this trait can't have any parameters.
    // But this is okay. The impl bakes them in, and user code works with the concrete table.

    fn new() -> Self where Self: Sized;
    fn domain() -> DomainName where Self: Sized;
    fn name() -> TableName where Self: Sized;
    fn guarantee() -> Guarantee where Self: Sized;
    // Do associated const parameters count as 'trait paramters'?

    fn prototype(&self) -> Box<TTable>;
    fn get_flush(&mut self) -> &mut Any;

    fn remove_rows(&mut self, &Universe, ::event::Event, tracking::SelectAny);
}
mopafy!(TTable);

/// A table held by `Universe`. Its information is used to populate concrete tables.
#[doc(hidden)]
pub struct GenericTable {
    pub domain: DomainName,
    pub name: TableName,
    pub columns: Vec<GenericColumn>,
    init_fns: Vec<fn(&Universe)>,
    pub guarantee: Guarantee,
    pub table: Box<TTable>,
}
#[doc(hidden)]
#[derive(Default, Clone)]
pub struct Guarantee {
    pub consistent: bool,
    pub sorted: bool,
}
impl GenericTable {
    pub fn new<T: TTable>(table: T) -> GenericTable {
        let domain = T::domain();
        let name = T::name();
        let guarantee = T::guarantee();
        intern::check_name(name.0);
        GenericTable {
            domain,
            name,
            columns: Vec::new(),
            init_fns: Vec::new(),
            guarantee,

            table: Box::new(table),
        }
    }

    pub fn add_init(&mut self, init: fn(&Universe)) {
        self.init_fns.push(init);
    }

    pub(crate) fn init(&self, universe: &Universe) {
        for init in &self.init_fns {
            init(universe);
        }
    }

    pub fn guard(self) -> RwLock<GenericTable> {
        RwLock::new(self)
    }

    /// Create a copy of this table with empty columns.
    pub fn prototype(&self) -> GenericTable {
        // FIXME: Just use clone()?
        GenericTable {
            domain: self.domain,
            name: self.name,
            columns: self.columns.iter().map(GenericColumn::prototype).collect(),
            init_fns: self.init_fns.clone(),
            guarantee: self.guarantee.clone(),

            table: self.table.prototype(),
        }
    }

    pub fn add_column(mut self, name: &'static str, type_name: &'static str, prototyper: Prototyper) -> Self {
        intern::check_name(name);
        for c in &self.columns {
            if c.name == name {
                panic!("Duplicate column name {}", name);
            }
        }
        self.columns.push(GenericColumn {
            name: name,
            stored_type_name: type_name,
            data: prototyper(),
            prototyper: prototyper,
        });
        self
    }

    pub fn get_column<C: Any>(&self, name: &str, type_name: &'static str) -> &C {
        let c = self.columns.iter().find(|c| c.name == name).unwrap_or_else(|| {
            panic!("Table {} doesn't have a {} column.", self.name, name);
        });
        if c.stored_type_name != type_name { panic!("Column {}/{} has datatype {:?}, not {:?}", self.name, name, c.stored_type_name, type_name); }
        match ::intern::desync_box(&c.data).downcast_ref() {
            Some(ret) => ret,
            None => {
                panic!("Column {}/{}: type conversion from {:?} to {:?} failed", self.name, name, c.stored_type_name, type_name);
            },
        }
    }

    pub fn get_column_mut<C: Any>(&mut self, name: &str, type_name: &'static str) -> &mut C {
        let my_name = &self.name;
        let c = self.columns.iter_mut().find(|c| c.name == name).unwrap_or_else(|| {
            panic!("Table {} doesn't have a {} column.", my_name, name);
        });
        if c.stored_type_name != type_name { panic!("Column {}/{} has datatype {:?}, not {:?}", self.name, name, c.stored_type_name, type_name); }
        match ::intern::desync_box_mut(&mut c.data).downcast_mut() {
            Some(ret) => ret,
            None => {
                panic!("Column {}/{}: type conversion from {:?} to {:?} failed", self.name, name, c.stored_type_name, type_name);
            },
        }
    }

    pub fn info(&self) -> String {
        let mut ret = format!("{}:", self.name);
        for col in &self.columns {
            ret.push_str(&format!(" {}:[{}]", col.name, col.stored_type_name));
        }
        ret
    }

    pub fn register(self) {
        use domain::{GlobalProperties, clone_globals};
        use std::collections::hash_map::Entry;
        let globals = clone_globals();
        let pmap: &mut GlobalProperties = &mut *globals.write().unwrap();
        match pmap.domains.get_mut(&self.domain) {
            None => panic!("Table {:?} registered before its domain {:?}", self.name, self.domain),
            Some(info) => {
                if super::domain::check_lock() && info.locked() {
                    panic!("Adding {}/{} to a locked domain\n", self.domain, self.name);
                }
                match info.tables.entry(self.name) {
                    Entry::Vacant(entry) => { entry.insert(self); },
                    Entry::Occupied(entry) => {
                        let entry = entry.get();
                        if !self.equivalent(entry) {
                            panic!("Tried to register {:?} on top of an existing table with different structure, {:?}", self, entry);
                        }
                    },
                }
            }
        }
    }

    fn equivalent(&self, other: &GenericTable) -> bool {
        return self.domain == other.domain
            && self.name == other.name
            && self.columns.len() == other.columns.len()
            && {
                for (a, b) in self.columns.iter().zip(other.columns.iter()) {
                    if a.name != b.name { return false; }
                    if a.stored_type_name != b.stored_type_name { return false; }
                }
                true
            };
    }
}
impl fmt::Debug for GenericTable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GenericTable [{}/{}] {:?}", self.domain, self.name, self.columns)
    }
}

#[doc(hidden)]
pub struct GenericColumn {
    name: &'static str,
    stored_type_name: &'static str,
    // "FIXME: PBox here is lame." -- What? No it isn't.
    data: PBox,
    prototyper: Prototyper,
}
impl fmt::Debug for GenericColumn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GenericColumn(name: {:?}, stored_type_name: {:?})", self.name, self.stored_type_name)
    }
}
impl GenericColumn {
    fn prototype(&self) -> GenericColumn {
        GenericColumn {
            name: self.name,
            stored_type_name: self.stored_type_name,
            data: (self.prototyper)(),
            prototyper: self.prototyper,
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TableName(pub &'static str);
impl fmt::Display for TableName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// FIXME: Rename. `TableRowId`?
#[doc(hidden)]
pub trait GetTableName: 'static + Send + Sync {
    /// The raw index, like `u32`.
    type Idx: 'static +
        ::num_traits::PrimInt +
        fmt::Display + fmt::Debug +
        ::std::hash::Hash + Copy + Ord
        + Send + Sync;

    fn get_domain() -> DomainName;
    fn get_name() -> TableName;
    fn get_guarantee() -> Guarantee;
    fn get_generic_table(&Universe) -> &RwLock<GenericTable>;
}

#[doc(hidden)]
pub trait LockedTable: Sized {
    type Row: GetTableName;
    fn len(&self) -> usize;
    fn is_deleted(&self, _idx: GenericRowId<Self::Row>) -> bool { false }
}

// FIXME: Why?
pub use ::assert_sorted::AssertSorted;
pub use ::index::*;
