use std::any::Any;
use std::sync::*;
use std::fmt;


pub use v11_macros::*;

#[macro_export]
define_invoke_proc_macro!(__v11_invoke_table);

/**

This macro generates a column-based data table.
(It is currently implemented using the procedural-masquerade hack.)

The syntax for this macro is:

```ignored
table! {
    #[kind = "…"]
    // table attributes can go here
    pub [DOMAIN/name_of_table] {
        // column attributes can go here
        column_name_1: [Element1; ColumnType1],
        column_name_2: [Element2; ColumnType2],
        column_name_3: [Element3; ColumnType3],
        // …
    }
}
```
where each ColumnType is a `TCol`, and Element is a `Storable` `<ColumnType as TCol>::Element`.

`pub` can be elided for a private table.

DOMAIN is specified using the `domain!` macro.

Here are some example columns:

* `[i32; VecCol<i32>]` (a column implemented with `Vec<i32>`)
* `[u8; SegCol<u8>]` (a column of u8 stored in non-contiguous chunks)
* `[bool; BoolCol]` (a column specialized for single bit storage)

(As a special convenience, `VecCol`, `SegCol`, and `BoolCol` are automatically `use`d by the macro.)

Table and column names must be valid Rust identifiers that also match the regex
`[A-Za-z][A-Za-z_0-9]*`.

Column elements must implement `Storable`.
Column types must implement `TCol`.

It is recommended that the table name be plural and the column name be singular,
eg in `customers.name[id]`.

# Using the Table

```ignored

// Create a new domain. This is a single-level namespace
domain! { MY_DOMAIN }

// Generate code for a table.
table! {
    #[kind = "append"]
    pub [MY_DOMAIN/my_table] {
        my_int: [i32; VecCol<i32>],
    }
}

fn main() {
    // Every domain, table, and property should be registered before creating the Universe.
    MY_DOMAIN::register();
    my_table::register();

    // Every member of MY_DOMAIN is initialized at this point.
    let universe = &Universe::new(&[MY_DOMAIN]);

    // The universe owns a `RwLock` for each table & property.
    let mut my_table = my_table::write(universe);
    my_table.push(my_table::Row {
        my_int: 42,
    });
}
```

# Table kinds and Guarantees
The 'kind' of a table selects what functions are generated and what guarantees are upheld.

## `#[kind = "consistent"]`
Rows in consistent tables can be used as *foreign keys* in other tables.
The main guarantee of the public table is that it is kept consistent with such tables:
the main row and references to it are deleted as a unit.

Since maintaining consistency requires locking other tables,
you must call `table.flush(universe)` instead of letting the table fall out of scope.
You also may call `table.no_flush()` to let someone else deal with it.

## `#[kind = "append"]`
Rows in an "append" table can not be removed. Consistency is thus trivially guaranteed.

## `#[kind = "sorted"]`
Guarantees the table is sorted. You must implement `Ord` for `$table::RowRef`.
If you put `#[sort_key]` on a column, it will do this for you.
(The macro derives `Eq`, `PartialEq`, and `PartialOrd` on `$table::RowRef`, and those + `Ord` on `$table::Row`)

Rows can be added with `merge`, and removed with `retain`.

Sorted tables are good for `joincore`.

## `#[kind = "bag"]`
NYI. (Row order would be arbitrary and there would be no consistency guarantee.)

## `#[kind = "list"]`
NYI. (Row order would remain intact, but there would be no consistency guarantee.)

# Using the generated table

A lock on the table must be obtained using `$tablename::read(universe)`.

(FIXME: Link to `v11::example`)

# Table Attributes

## `#[row_id = "usize"]`
Sets what the (underlying) primitive is used for indexing the table. The default is `usize`.
This is useful when this table is going to have foreign keys pointing at it.
You may also want to set this to an explicitly sized integer if the table is to be serialized.

## `#[row_derive(Foo, Bar)]`
Puts `#[derive(Foo, Bar)]` on the generated `Row` and `RowRef` structs.

## `#[version = "0"]`
A version number. The default is `0`.

# Column Attributes

## `#[foreign]`
The row's element must be another table's RowId.
This generates a `struct track_$COL_events`, for which `Tracker` must be implemented, to react to structural events on the foreign table.

## `#[foreign_auto]`
This automatically implements `Tracker`. Rows corresponding to deleted foreign rows will be removed.
This requires `#[index]` or `#[sort_key]`.

## `#[index]`
Creates an index of the column, using a `BTreeMap`.
Indexed elements are immutable.

## `#[sort_key]`
Use the element's comparision order to derive `Ord` for `RowRef`.
**/
// (FIXME: lang=ignored=lame)
#[macro_export]
macro_rules! table {
    (
        $(#[$meta:meta])*
        [$domain:ident/$name:ident]
        $($args:tt)*
    ) => {
        // It'd be nicer to generate 'mod' in the procmacro, but the procedural masquerade hack
        // can't be invoked twice in the same module.
        #[allow(dead_code)]
        mod $name {
            __v11_invoke_table! {
                __v11_internal_table!($(#[$meta])* [$domain/$name] $($args)*)
            }
        }
    };
    (
        $(#[$meta:meta])*
        pub [$domain:ident/$name:ident]
        $($args:tt)*
    ) => {
        #[allow(dead_code)]
        pub mod $name {
            __v11_invoke_table! {
                __v11_internal_table!($(#[$meta])* [$domain/$name] $($args)*)
            }
        }
    };
}

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

use std::collections::btree_map;
use tracking::Tracker;


pub trait TTable: ::mopa::Any + Send + Sync {
    fn new() -> Self where Self: Sized;
    fn domain() -> DomainName where Self: Sized;
    fn name() -> TableName where Self: Sized;
    fn guarantee() -> Guarantee where Self: Sized;

    fn prototype(&self) -> Box<TTable>; // Can't implement this the obvious way. :|
}
mopafy!(TTable);

pub(crate) type FreeList = btree_map::BTreeMap<usize, ()>;
pub(crate) type FreeKeys<'a> = btree_map::Keys<'a, usize, ()>;

/// A table held by `Universe`. Its information is used to populate concrete tables.
#[doc(hidden)]
pub struct GenericTable {
    pub domain: DomainName,
    pub name: TableName,
    pub columns: Vec<GenericColumn>,
    init_fns: Vec<fn(&Universe)>,
    // All the other fields don't need locks, but this one does because we need to continue holding
    // it after releasing the lock on `GenericTable`.
    pub trackers: Arc<RwLock<Vec<Box<Tracker + Send + Sync>>>>,
    pub(crate) no_trackers: bool,
    pub(crate) delete: Vec<usize>,
    pub(crate) add: Vec<usize>,
    pub cleared: bool,
    pub free: FreeList,
    pub need_flush: bool,
    pub guarantee: Guarantee,
    pub sort_events: bool,
    pub table: Box<TTable>,
}
#[doc(hidden)]
#[derive(Default, Clone)]
pub struct Guarantee {
    pub consistent: bool,
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
            trackers: Default::default(),
            no_trackers: true,
            init_fns: Vec::new(),
            guarantee,

            delete: Vec::new(),
            add: Vec::new(),
            cleared: false,
            free: Default::default(),
            need_flush: false,
            sort_events: false,
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
            trackers: Arc::clone(&self.trackers),
            no_trackers: self.no_trackers,
            init_fns: self.init_fns.clone(),
            guarantee: self.guarantee.clone(),

            delete: Vec::new(),
            add: Vec::new(),
            free: Default::default(),
            cleared: false,
            need_flush: false,
            sort_events: self.sort_events,
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

#[doc(hidden)]
pub trait GetTableName {
    type Idx: ::num_traits::PrimInt + fmt::Display + fmt::Debug + ::std::hash::Hash + Copy;
    fn get_domain() -> DomainName;
    fn get_name() -> TableName;
}

#[doc(hidden)]
pub trait LockedTable: Sized {
    type Row: GetTableName;
    fn len(&self) -> usize;
    fn is_deleted(&self, _idx: GenericRowId<Self::Row>) -> bool { false }
}

pub use ::assert_sorted::AssertSorted;
pub use ::index::*;
