#![macro_use]

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

# Using the Table

```ignored
# FIXME: lang=ignored=lame

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
the main row and its linkages are (with some user-provided implementation!) deleted as a unit.

Since maintaining consistency requires locking other tables,
you must call `table.flush(universe)` instead of letting the table fall out of scope.
You also may call `table.no_flush()` to let someone else deal with it.

## `#[kind = "append"]`

Rows in an "append" table can not be removed. Consistency is thus trivially guaranteed.

## `#[kind = "bag"]`
NYI. (Internal order would be arbitrary and there would be no consistency guarantee.)

## `#[kind = "sorted"]`
NYI.

# Using the generated table

A lock on the table must be obtained using `$tablename::read(universe)`.

(FIXME: Link to `cargo doc` of a sample project. In the meantime, uh, check out `tests/tables.rs` I guess.)

# Table Attributes

## `#[row_id = "usize"]`
Sets what the (underlying) primitive is used for indexing the table. The default is `usize`.
This is useful when this table is going to have foreign keys pointing at it.

## `#[row_derive(Foo, Bar)]`
Puts `#[derive(Foo, Bar)]` on the generated `Row` and `RowRef` structs.

# Column Attributes

## `#[foreign]`
The row's element must be another table's RowId.

## `#[index]`
Creates an index of the column, using a `BTreeMap`.
Indexed elements are immutable.

FIXME: IndexSet. Or just expose a set() method.

<hr>

// FIXME: Would `#[mut]` be cool? Cols are immutable by default? :D


<hr>

# OLD

This macro generates a column-based data table.
(It is currently implemented using the procedural-masquerade hack.)

The syntax for this macro is:

```ignored
table! {
    pub [DOMAIN/name_of_table] {
        column_name_1: [Element1; ColumnType1],
        column_name_2: [Element2; ColumnType2],
        column_name_3: [Element3; ColumnType3],
        // ...
    }
    // optonal table attribute section
    impl {
        table_attributes;
        of_which_there_are(ManyVarieties);
    }
}
```
where each `ColumnType` is a `TCol`, and Element is `<ColumnType as TCol>::Element`.

For example:


* `[u8; SegCol<u8>]`
* `[bool; BoolCol]`


(Note that `VecCol`, `SegCol`, and `BoolCol` are already `use`d by the macro for your convenience.)

`DOMAIN`s are declared using the `domain!` macro.
The leading `pub` may be elided to make the generated module private.

Table and column names must be valid Rust identifiers that also match the regex
`[A-Za-z][A-Za-z_0-9]*`.

Column elements must implement `Storable`.
Column types must implement `TCol`.

The `impl` section is optional.

# Using the generated table
(FIXME: Link to `cargo doc` of a sample project. In the meantime, uh, check out `tests/tables.rs` I guess.)


# Table Attributes
These are options that affect the code generation of tables. (And are why we can't just use `macro_rules!`!)
Notice that each attribute is terminated by a `;`.

## `RowId = some_primitive;`
Sets what the (underlying) primitive is used for indexing the table. The default is `usize`.
This is useful when this table is going to have foreign keys pointing at it.

## `NoDebug;`
`#[derive(Debug)]` is added to `Row` by default; this prevents that.

## `NoComplexMut;`
Don't provide the mut functions `filter` and `visit`.

## `NoCopy;`
Don't derive Copy on `Row` (but DO derive Clone).

## `NoClone;`
Derive neither Clone nor Copy on `Row`.

## `Track;`
Creates an event log of rows that were moved, deleted, and removed.
Dependants of this table, `Tracker`s, are notified of these changes by calling `flush()`.

To avoid the error of the event log being forgotten, the lock on a changed table will panic
if a `flush()` should be called. This can be surpressed by calling `noflush()`.
(FIXME: unfinished; need #[foreign])

## `GenericSort;`
Adds a parameterized sort method.

## `SortBy(SomeColumnName);`
Add a method to sort the table by that column.
(FIXME: #[sortable] on the column?)

## `FreeList;`
(FIXME: nyi. Also tricky.)
Allow marking rows as dead, and pushing new rows will re-use a dead row.

## `Save;`
Add methods for encoding or decoding the table by row (or column), using rustc.
(FIXME: Maybe it should be default? Also, serde.)

## `Static;`
Marks the table as being something that is only modified once.
This allows skipping some codegen.
It also makes using certain other attributes an error.
Static tables will want to to use `VecCol` columns rather than `SegCol` columns.
(FIXME: Initialize the table with a function; then locking is unnecessary. [But, yikes on implementing that])

## `Version(number);`
Sets the version number of the table. This is a `usize`. Its default value is `1`.

## `Legacy(number);`
Equivalent to `Version` and `Static`.

**/
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
    pub fn get_generic_table(&self, domain_id: DomainId, name: TableName) -> &RwLock<GenericTable> {
        use domain::MaybeDomain;
        if let Some(&MaybeDomain::Domain(ref domain)) = self.domains.get(domain_id.0) {
            return domain.get_generic_table(name);
        }
        panic!("Request for table {} in unknown domain #{}", name, domain_id.0);
    }

    pub fn table_names(&self) -> Vec<TableName> {
        let mut ret = Vec::new();
        for domain in &self.domains {
            if let MaybeDomain::Domain(ref domain) = *domain {
                for table in domain.tables.keys() {
                    ret.push(*table);
                }
            }
        }
        ret
    }
}

type Prototyper = fn() -> PBox;

use std::collections::BTreeMap;
use tracking::Tracker;

/// A table held by `Universe`. Its information is used to populate concrete tables.
#[doc(hidden)]
pub struct GenericTable {
    pub domain: DomainName,
    pub name: TableName,
    pub columns: Vec<GenericColumn>,
    init_fns: Vec<fn(&Universe)>,
    // All the other fields don't need locks, but this one does because it can out-last.
    pub trackers: Arc<RwLock<Vec<Box<Tracker + Send + Sync>>>>,
    pub(crate) no_trackers: bool,
    pub(crate) delete: Vec<usize>,
    pub(crate) add: Vec<usize>,
    pub cleared: bool,
    pub free: BTreeMap<usize, ()>,
    pub need_flush: bool,
}
impl GenericTable {
    pub fn new(domain: DomainName, name: TableName) -> GenericTable {
        intern::check_name(name.0);
        GenericTable {
            domain: domain,
            name: name,
            columns: Vec::new(),
            trackers: Default::default(),
            no_trackers: true,
            init_fns: Vec::new(),

            delete: Vec::new(),
            add: Vec::new(),
            cleared: false,
            free: BTreeMap::new(),
            need_flush: false,
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
        GenericTable {
            domain: self.domain,
            name: self.name,
            columns: self.columns.iter().map(GenericColumn::prototype).collect(),
            trackers: Arc::clone(&self.trackers),
            no_trackers: self.no_trackers,
            init_fns: self.init_fns.clone(),

            delete: Vec::new(),
            add: Vec::new(),
            free: BTreeMap::new(),
            cleared: false,
            need_flush: false,
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

pub trait GetTableName {
    type Idx: ::num_traits::PrimInt + fmt::Display + fmt::Debug + ::std::hash::Hash + Copy;
    fn get_domain() -> DomainName;
    fn get_name() -> TableName;
}

pub trait LockedTable: Sized {
    type Row: GetTableName;
    fn len(&self) -> usize;
    fn is_deleted(&self, _idx: GenericRowId<Self::Row>) -> bool { false }
}

pub use ::assert_sorted::AssertSorted;
pub use ::index::*;
