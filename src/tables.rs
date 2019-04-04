use std::any::Any;
use std::sync::*;
use std::fmt;
use std::borrow::Cow;

use serde::ser::{Serialize};
use serde::de::{DeserializeOwned};

pub use v11_macros::*;

use Universe;
use intern;
use domain::{DomainName, DomainId, MaybeDomain};
use columns::AnyCol;

impl Universe {
    #[doc(hidden)]
    pub fn get_generic_table(&self, domain_id: DomainId, name: TableName) -> &RwLock<GenericTable> {
        if let Some(&MaybeDomain::Domain(ref domain)) = self.domains.get(domain_id.0) {
            return domain.get_generic_table(name);
        }
        panic!("Request for table {} in unknown domain #{}", name, domain_id.0);
    }
}


/// A function that creates a new `GenericColumn`.
pub type Prototyper = fn() -> GenericColumn;


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
    fn get_flush_ref(&self) -> &Any;
    fn get_flush_mut(&mut self) -> &mut Any;

    fn get_row_remover(&self) -> fn(&Universe, Event, SelectAny);

    /// Returns a `$table::Extraction`, if the table supports serialization.
    /// The `Extraction` type is accessible in generic contexts via
    /// `<$table::Row as SerialExtraction>::Extract`.
    fn extract_serialization(
        &self,
        universe: &Universe,
        selection: tracking::SelectAny,
    ) -> Option<Box<::erased_serde::Serialize>>;
}
mopafy!(TTable);

/// A table held by `Universe`. Its information is used to populate concrete tables.
#[doc(hidden)]
pub struct GenericTable {
    pub domain: DomainName,
    pub name: TableName,
    pub schema_version: u32,
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
    pub append_only: bool,
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
            schema_version: 0, // FIXME
            columns: Vec::new(),
            init_fns: Vec::new(),
            guarantee,

            table: Box::new(table),
        }
    }

    pub fn add_init(&mut self, init: fn(&Universe)) {
        self.init_fns.push(init);
    }

    pub(crate) fn get_inits(&self) -> Vec<fn(&Universe)> {
        self.init_fns.clone()
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
            schema_version: self.schema_version,
            columns: self.columns.iter().map(|c| (c.prototyper)()).collect(),
            init_fns: self.init_fns.clone(),
            guarantee: self.guarantee.clone(),

            table: self.table.prototype(),
        }
    }

    pub fn add_column(mut self, prototyper: Prototyper) -> Self {
        let col = prototyper();
        intern::check_name(&col.meta.name);
        for c in &self.columns {
            if c.meta.name == col.meta.name {
                panic!("Duplicate column name {}", col.meta.name);
            }
        }
        self.columns.push(col);
        self
    }

    pub fn get_column<C>(
        &self,
        name: &str,
        type_name: &'static str,
    ) -> &C
    where C: Any + Send + Sync
    {
        let c = self.columns.iter().find(|c| c.meta.name == name).unwrap_or_else(|| {
            panic!("Table {} doesn't have a {} column.", self.name, name);
        });
        if c.meta.stored_type_name != type_name { panic!("Column {}/{} has datatype {:?}, not {:?}", self.name, name, c.meta.stored_type_name, type_name); }
        let cdata: &AnyCol = &*c.data;
        match cdata.downcast_ref() {
            Some(ret) => ret,
            None => {
                panic!("Column {}/{}: type conversion from {:?} to {:?} failed", self.name, name, c.meta.stored_type_name, type_name);
            },
        }
    }

    pub fn get_column_mut<C>(
        &mut self,
        name: &str,
        type_name: &'static str,
    ) -> &mut C
    where C: Any + Send + Sync
    {
        let my_name = &self.name;
        let c = self.columns.iter_mut().find(|c| c.meta.name == name).unwrap_or_else(|| {
            panic!("Table {} doesn't have a {} column.", my_name, name);
        });
        if c.meta.stored_type_name != type_name { panic!("Column {}/{} has datatype {:?}, not {:?}", self.name, name, c.meta.stored_type_name, type_name); }
        let cdata: &mut AnyCol = &mut *c.data;
        match cdata.downcast_mut() {
            Some(ret) => ret,
            None => {
                panic!("Column {}/{}: type conversion from {:?} to {:?} failed", self.name, name, c.meta.stored_type_name, type_name);
            },
        }
    }

    pub fn info(&self) -> String {
        let mut ret = format!("{}:", self.name);
        for col in &self.columns {
            ret.push_str(&format!(" {}:[{}]", col.meta.name, col.meta.stored_type_name));
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
                    Entry::Vacant(entry) => {
                        info.tables_registration_order.push(self.name);
                        entry.insert(self);
                    },
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
                    if a.meta != b.meta { return false; }
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
    pub meta: ColumnMeta,
    // "FIXME: PBox here is lame." -- What? No it isn't.
    pub data: Box<AnyCol>,
    pub prototyper: Prototyper,
}
impl fmt::Debug for GenericColumn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.meta)
    }
}

#[derive(Debug, Clone, PartialEq)]
#[derive(Serialize, Deserialize)]
pub struct ColumnMeta {
    pub name: Cow<'static, str>,
    pub stored_type_name: Cow<'static, str>,
    pub version: usize,
}



#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Serialize, Deserialize)]
pub struct TableName(pub &'static str);
impl fmt::Display for TableName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// FIXME: Rename. `TableRowId`? Difficult to say. `TableIdent`?
#[doc(hidden)]
pub trait GetTableName: 'static + Send + Sync {
    /// The raw index, like `u32`.
    type Idx: 'static +
        ::num_traits::PrimInt + ::num_traits::FromPrimitive +
        fmt::Display + fmt::Debug +
        ::std::hash::Hash + Copy + Ord
        + Send + Sync
        + Serialize + DeserializeOwned;

    fn get_domain() -> DomainName;
    fn get_name() -> TableName;
    fn get_guarantee() -> Guarantee;
    fn get_generic_table(&Universe) -> &RwLock<GenericTable>;
    fn new_generic_table() -> GenericTable;
}

use event::Event;
use tracking::SelectAny;
pub trait SerialExtraction: GetTableName {
    type Extraction: Serialize + DeserializeOwned;

    fn extract(universe: &Universe, selection: SelectAny) -> Self::Extraction;
    fn restore(universe: &Universe, extraction: Self::Extraction, event: Event) -> Result<(), &'static str>;
}

#[doc(hidden)]
pub trait LockedTable: Sized {
    type Row: GetTableName;
    fn len(&self) -> usize;
    fn is_deleted(&self, _idx: GenericRowId<Self::Row>) -> bool { false }
    fn delete_row(&mut self, _idx: GenericRowId<Self::Row>) { unimplemented!("LockedTable::delete") }
}

// FIXME: Why?
pub use ::assert_sorted::AssertSorted;
pub use ::index::*;
