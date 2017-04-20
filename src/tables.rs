#![macro_use]

use std::any::Any;
use std::sync::*;


pub use v11_macros::*;

#[macro_export]
define_invoke_proc_macro!(__v11_invoke_table);

/**
This macro + build script generates a column-based data table.
It is implemented using the procedural-masquerade hack.

The simplified syntax for this macro is:

```ignored
table! {
    [DOMAIN/name_of_table] {
        column_name_1: [Element1; ColumnType1],
        column_name_2: [Element2; ColumnType2],
        column_name_3: [Element3; ColumnType3],
        // ...
    }
}
```
where ColumnTypeN is a `TCol`, and ElementN is `<ColumnTypeN as TCol>::Element`, for example `[u8; SegCol<u8>]`
or `[bool; BoolCol]`.



Table and column names must be valid Rust identifiers that also match the regex
`[A-Za-z][A-Za-z_0-9]*`.

Column elements must implement `Storable`.
Column types must implement `TCol`.

# Advanced Usage

The full syntax for this macro is:

```ignored
table! {
    pub [DOMAIN/name_of_table] {
        column_name: [element_type; ColumnType<element_type>],
    }
    impl {
        table_attributes;
        of_which_there_are(ManyVarieties);
    }
    mod {
        // user code that gets copied to the end of the table.rs
        // Perhaps we could have something like...
        impl<'u> Read<'u> {
            fn read_me(self) {}
        }
    }
}
```

The `impl` and `mod` sections are both optional, but must be given in that order.
The leading `pub` may be elided to make the module private.

# Table Attributes
These are options that affect the code generation of tables (and are why we can't just use `macro_rules!`).
Notice that each attribute is terminated by a `;`.

## `RowId = some_primitive;`
Sets what the (underlying) primitive is used for indexing the table. The default is `usize`.
This is useful when this table is going to have foreign keys pointing at it.

## `NoDebug;`
`#[derive(Debug)]` is added to `Row` by default; this prevents that.

## `TrackRm;`
(FIXME: nyi)
Keep track of which rows have been removed by `filter` and `visit`.

## `ForeignCascade;`
(FIXME: nyi)
If a foreign key to a table that has `TrackRm` becomes invalidated by removing the refered to row,
then that row (on this table) will be removed by calling (FIXME: some function).

## `TrackModify;`
(FIXME: nyi. Also tricky.)
Keeps a sparse list of modified rows.

## `GenericSort;`
Adds a parameterized sort method.

## `SortBy(SomeColumnName);`
Add a method to sort the table by that column.

## `FreeList;`
(FIXME: nyi. Also tricky.)
Allow marking rows as dead, and pushing new rows will pick a dead row.

## `Save;`
Add methods for encoding or decoding the table by row (or column), using rustc.

## `Static;`
Marks the table as being something that is only modified once.
This allows skipping some codegen.
It also makes using certain other attributes an error.
Static tables will want to to use `VecCol` columns rather than `SegCol` columns.

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
        $(#[$meta])*
        mod $name {
            table!(mod $domain/$name $($args)*);
        }
    };
    (
        $(#[$meta:meta])*
        pub [$domain:ident/$name:ident]
        $($args:tt)*
    ) => {
        $(#[$meta])*
        pub mod $name {
            table!(mod $domain/$name $($args)*);
        }
    };
    (mod $domain:ident/$name:ident $($args:tt)*) => {
        #[allow(unused_imports)]
        use super::*;
        use super::$domain as TABLE_DOMAIN;

        __v11_invoke_table! {
            __v11_internal_table!($domain/$name $($args)*)
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
        panic!("Request for table {} in unknown domain", name);
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

/// A table held by `Universe`. Its information is used to populate concrete tables.
#[derive(Debug)]
pub struct GenericTable {
    pub domain: DomainName,
    pub name: TableName,
    pub columns: Vec<GenericColumn>,
}
impl GenericTable {
    pub fn new(domain: DomainName, name: TableName) -> GenericTable {
        intern::check_name(name.0);
        GenericTable {
            domain: domain,
            name: name,
            columns: Vec::new(),
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
        }
    }

    pub fn add_column(mut self, name: &'static str, type_name: &'static str, prototyper: Prototyper) -> Self {
        // Why is the 'static necessary??? Does it refer to the vtable or something?
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
        use domain::{GlobalProperties, PROPERTIES};
        use std::collections::hash_map::Entry;
        let mut pmap: &mut GlobalProperties = &mut *PROPERTIES.write().unwrap();
        match pmap.domains.get_mut(&self.domain) {
            None => panic!("Table {:?} registered before its domain {:?}", self.name, self.domain),
            Some(mut info) => match info.tables.entry(self.name) {
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

pub struct GenericColumn {
    name: &'static str,
    stored_type_name: &'static str,
    // FIXME: PBox here is lame.
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


// indexing



use std::marker::PhantomData;
use num_traits::PrimInt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TableName(pub &'static str);
impl fmt::Display for TableName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub trait GetTableName {
    fn get_name() -> TableName;
}

#[derive(Copy, Clone)]
pub struct GenericRowId<I: PrimInt, T: GetTableName> {
    #[doc(hidden)]
    pub i: I,
    #[doc(hidden)]
    pub t: PhantomData<T>,
}
impl<I: PrimInt, T: GetTableName> GenericRowId<I, T> {
    pub fn new(i: I) -> Self {
        GenericRowId {
            i: i,
            t: PhantomData,
        }
    }

    pub fn to_usize(&self) -> usize { self.i.to_usize().unwrap() }
    pub fn to_raw(&self) -> I { self.i }
    pub fn next(&self) -> Self {
        Self::new(self.i + I::one())
    }
}
impl<I: PrimInt, T: GetTableName> Default for GenericRowId<I, T> {
    fn default() -> Self {
        GenericRowId {
            i: I::max_value() /* UNDEFINED_INDEX */,
            t: PhantomData,
        }
    }
}

use std::fmt;
impl<I: PrimInt + fmt::Display, T: GetTableName> fmt::Debug for GenericRowId<I, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}[{}]", T::get_name().0, self.i)
    }
}

#[test]
fn test_formatting() {
    struct TestName;
    impl GetTableName for TestName {
        fn get_name() -> TableName { TableName("test_table") }
    }
    let gen: GenericRowId<usize, TestName> = GenericRowId {
        i: 23,
        t: ::std::marker::PhantomData,
    };
    assert_eq!("test_table[23]", format!("{:?}", gen));
}


use std::cmp::{Eq, PartialEq, PartialOrd, Ord};
impl<I: PrimInt, T: GetTableName> PartialEq for GenericRowId<I, T> {
    fn eq(&self, other: &GenericRowId<I, T>) -> bool {
        self.i == other.i
    }
}
impl<I: PrimInt, T: GetTableName> Eq for GenericRowId<I, T> {}
impl<I: PrimInt, T: GetTableName> PartialOrd for GenericRowId<I, T> {
    fn partial_cmp(&self, other: &GenericRowId<I, T>) -> Option<::std::cmp::Ordering> {
        self.i.partial_cmp(&other.i)
    }
}
impl<I: PrimInt, T: GetTableName> Ord for GenericRowId<I, T> {
    fn cmp(&self, other: &GenericRowId<I, T>) -> ::std::cmp::Ordering {
        self.i.cmp(&other.i)
    }
}

// Things get displeasingly manual due to the PhantomData.
use std::hash::{Hash, Hasher};
impl<I: PrimInt + Hash, T: GetTableName> Hash for GenericRowId<I, T> {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.i.hash(state);
    }
}

use rustc_serialize::{Encoder, Encodable, Decoder, Decodable};
impl<I: PrimInt + Encodable, T: GetTableName> Encodable for GenericRowId<I, T> {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        self.i.encode(s)
    }
}

impl<I: PrimInt + Decodable, T: GetTableName> Decodable for GenericRowId<I, T> {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        Ok(Self::new(try!(I::decode(d))))
    }
}
