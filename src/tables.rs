#![macro_use]

use std::any::Any;
use std::sync::*;


/**
This macro + build script generates a column-based data table.

The simplified syntax for this macro is:

```ignored
table! {
    pub name_of_table {
        column_name: [element_type; ColumnType<element_type>],
    }
    mod {
        use path::to::element_type;
    }
}
```

The actual macro just includes the module generated by your `build.rs` script. See
`v11_test/build.rs` & `v11_test/Cargo.toml` for an example of how to set this up. (It's super-easy.)

Table and column names must be valid Rust identifiers that also match the regex
"[A-Za-z][A-Za-z_0-9]*".

Column elements must implement `Storable`.
Column types must implement `TCol`.

# Advanced Usage

The full syntax for this macro is:

```ignored
table! {
    pub name_of_table {
        column_name: [element_type; ColumnType<element_type>],
    }
    impl {
        some_special_settings;
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

# `RowId = some_primitive;`
Sets what the (underlying) primitive is used for indexing the table. The default is `usize`.
This is useful for foreign keys.

# `NoDebug;`
`#[derive(Debug)]` is added to `Row` by default.

# `TrackRm;`
(FIXME: nyi)
Keep track of which rows have been removed by `filter` and `visit`.

# `ForeignCascade;`
(FIXME: nyi)
If a foreign key to a table that has `TrackRm` becomes invalidated by removing the refered to row,
then that row (on this table) will be removed by calling (FIXME: some function).

# `TrackModify;`
(FIXME: nyi. Also tricky.)
Keeps a sparse list of modified rows.

# `GenericSort;`
Adds a parameterized sort method.

# `SortBy(SomeColumnName);`
Sorts the table by that column.

# `FreeList;`
(FIXME: nyi. Also tricky.)
Allow marking rows as dead, and pushing new rows will pick a dead row.

# `Encode(method);` and `Decode(method);`
(FIXME: Serde nyi)
method can be `Serde` or `Rustc`. Adds methods for encoding or decoding the table.

# `Static;`
Marks the table as being something that is only modified once.
This allows skipping some codegen.

 **/
#[macro_export]
macro_rules! table {
    (pub $domain:ident/$name:ident $($args:tt)*) => {
        pub mod $name {
            use super::$domain as TABLE_DOMAIN;
            include!(concat!(
                env!("OUT_DIR"),
                "/v11_generated_tables/",
                stringify!($name),
                ".rs",
            ));
        }
    };
    ($domain:ident/$name:ident $($args:tt)*) => {
        mod $name {
            use super::$domain as TABLE_DOMAIN;
            include!(concat!(
                env!("OUT_DIR"),
                "/v11_generated_tables/",
                stringify!($name),
                ".rs",
            ));
        }
    };
}

use Universe;
use intern;
use intern::PBox;
use domain::DomainName;

impl Universe {
    pub fn add_table(&mut self, table: GenericTable) {
        if table.columns.is_empty() { panic!("Table has no columns"); }
        use std::collections::hash_map::Entry;
        match self.tables.entry(table.name.clone()) {
            Entry::Occupied(e) => {
                e.get().read().unwrap().assert_mergable(&table);
                // Maybe it's not unreasonable to be able to add columns.
            },
            Entry::Vacant(e) => { e.insert(RwLock::new(table)); },
        }
    }

    pub fn get_generic_table<'u, 's>(&'u self, name: &'s str) -> &'u RwLock<GenericTable> {
        match self.tables.get(name) {
            None => panic!("Table {} was not registered", name),
            Some(t) => t,
        }
    }
}

/// A table held by `Universe`. Its information is used to populate concrete tables.
pub struct GenericTable {
    pub domain: DomainName,
    pub name: String,
    pub columns: Vec<GenericColumn>,
}
impl GenericTable {
    pub fn new(domain: DomainName, name: &str) -> GenericTable {
        intern::check_name(name);
        GenericTable {
            domain: domain,
            name: name.to_string(),
            columns: Vec::new(),
        }
    }

    pub fn add_column(mut self, name: &str, type_name: &'static str, inst: PBox) -> Self {
        // Why is the 'static necessary??? Does it refer to the vtable or something?
        intern::check_name(name);
        for c in &self.columns {
            if c.name == name {
                panic!("Duplicate column name {}", name);
            }
        }
        self.columns.push(GenericColumn {
            name: name.to_string(),
            data: inst,
            stored_type_name: type_name,
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

    fn assert_mergable(&self, other: &GenericTable) {
        if self.name != other.name {
            panic!("Mismatched table names: {:?} vs {:?}", self.name, other.name);
        }
        let crash = || {
            panic!("Tried to register table {} with incompatible columns!\nOld table: {}\nNew table: {}\n", other.name, self.info(), other.info());
        };
        if self.columns.len() != other.columns.len() {
            crash();
        }
        for (a, b) in self.columns.iter().zip(other.columns.iter()) {
            if a.name != b.name { crash(); }
            if a.stored_type_name != b.stored_type_name { crash(); }
        }
    }

    pub fn info(&self) -> String {
        let mut ret = format!("{}:", self.name);
        for col in &self.columns {
            ret.push_str(&format!(" {}:[{}]", col.name, col.stored_type_name));
        }
        ret
    }
}

pub struct GenericColumn {
    name: String,
    stored_type_name: &'static str,
    data: PBox,
}


// indexing



use std::marker::PhantomData;
use num_traits::PrimInt;

pub trait TableName {
    fn get_name() -> &'static str;
}

#[derive(Copy, Clone)]
pub struct GenericRowId<I: PrimInt, T: TableName> {
    #[doc(hidden)]
    pub i: I,
    #[doc(hidden)]
    pub t: PhantomData<T>,
}
impl<I: PrimInt, T: TableName> GenericRowId<I, T> {
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
impl<I: PrimInt, T: TableName> Default for GenericRowId<I, T> {
    fn default() -> Self {
        GenericRowId {
            i: I::max_value() /* UNDEFINED_INDEX */,
            t: PhantomData,
        }
    }
}

use std::fmt;
impl<I: PrimInt + fmt::Display, T: TableName> fmt::Debug for GenericRowId<I, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}[{}]", T::get_name(), self.i)
    }
}

#[test]
fn test_formatting() {
    struct TestName;
    impl TableName for TestName {
        fn get_name() -> &'static str { "test_table" }
    }
    let gen: GenericRowId<usize, TestName> = GenericRowId {
        i: 23,
        t: ::std::marker::PhantomData,
    };
    assert_eq!("test_table[23]", format!("{:?}", gen));
}


use std::cmp::{Eq, PartialEq, PartialOrd, Ord};
impl<I: PrimInt, T: TableName> PartialEq for GenericRowId<I, T> {
    fn eq(&self, other: &GenericRowId<I, T>) -> bool {
        self.i == other.i
    }
}
impl<I: PrimInt, T: TableName> Eq for GenericRowId<I, T> {}
impl<I: PrimInt, T: TableName> PartialOrd for GenericRowId<I, T> {
    fn partial_cmp(&self, other: &GenericRowId<I, T>) -> Option<::std::cmp::Ordering> {
        self.i.partial_cmp(&other.i)
    }
}
impl<I: PrimInt, T: TableName> Ord for GenericRowId<I, T> {
    fn cmp(&self, other: &GenericRowId<I, T>) -> ::std::cmp::Ordering {
        self.i.cmp(&other.i)
    }
}

// Things get displeasingly manual due to the PhantomData.
use std::hash::{Hash, Hasher};
impl<I: PrimInt + Hash, T: TableName> Hash for GenericRowId<I, T> {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.i.hash(state);
    }
}

use rustc_serialize::{Encoder, Encodable, Decoder, Decodable};
impl<I: PrimInt + Encodable, T: TableName> Encodable for GenericRowId<I, T> {
    fn encode<S: Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        self.i.encode(s)
    }
}

impl<I: PrimInt + Decodable, T: TableName> Decodable for GenericRowId<I, T> {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        Ok(Self::new(try!(I::decode(d))))
    }
}
