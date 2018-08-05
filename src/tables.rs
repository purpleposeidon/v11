use std::any::Any;
use std::sync::*;
use std::fmt;

use serde::Serialize;

pub use v11_macros::*;

use Universe;
use intern;
use domain::{DomainName, DomainId, MaybeDomain};
use columns::AnyCol;
use tracking::SelectAny;

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
// FIXME: SmallBox!
pub type BoxedSerialize<'a> = Box<::erased_serde::Serialize + 'a>;
/// A function that, given a generic column and a selection, returns a `Serialize`able object that
/// can serialize that selection.
pub type SelectionSerializerFactory = for<'a> fn(&'a GenericColumn, &'a SelectAny) -> BoxedSerialize<'a>;
pub fn no_serializer_factory(col: &GenericColumn, _: &SelectAny) -> BoxedSerialize<'static> {
    panic!("Column {:?} can't be serialized", col.name)
}


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

    fn serial_selection<'a>(&self, &'a tracking::SelectAny) -> BoxedSerialize<'a>;
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
            columns: self.columns.iter().map(|c| (c.prototyper)()).collect(),
            init_fns: self.init_fns.clone(),
            guarantee: self.guarantee.clone(),

            table: self.table.prototype(),
        }
    }

    pub fn add_column(mut self, prototyper: Prototyper) -> Self {
        let col = prototyper();
        intern::check_name(col.name);
        for c in &self.columns {
            if c.name == col.name {
                panic!("Duplicate column name {}", col.name);
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
        let c = self.columns.iter().find(|c| c.name == name).unwrap_or_else(|| {
            panic!("Table {} doesn't have a {} column.", self.name, name);
        });
        if c.stored_type_name != type_name { panic!("Column {}/{} has datatype {:?}, not {:?}", self.name, name, c.stored_type_name, type_name); }
        let cdata: &AnyCol = &*c.data;
        match cdata.downcast_ref() {
            Some(ret) => ret,
            None => {
                panic!("Column {}/{}: type conversion from {:?} to {:?} failed", self.name, name, c.stored_type_name, type_name);
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
        let c = self.columns.iter_mut().find(|c| c.name == name).unwrap_or_else(|| {
            panic!("Table {} doesn't have a {} column.", my_name, name);
        });
        if c.stored_type_name != type_name { panic!("Column {}/{} has datatype {:?}, not {:?}", self.name, name, c.stored_type_name, type_name); }
        let cdata: &mut AnyCol = &mut *c.data;
        match cdata.downcast_mut() {
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
    pub name: &'static str,
    pub stored_type_name: &'static str,
    // "FIXME: PBox here is lame." -- What? No it isn't.
    pub data: Box<AnyCol>,
    pub prototyper: Prototyper,
    pub serializer_factory: SelectionSerializerFactory,
}
impl fmt::Debug for GenericColumn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "GenericColumn(name: {:?}, stored_type_name: {:?})", self.name, self.stored_type_name)
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Serialize, Deserialize)]
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
        + Send + Sync
        + Serialize;

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
