
#[macro_export]
define_invoke_proc_macro!(__v11_invoke_table);

/**

This macro generates a column-based data table.
(It is currently implemented using the procedural-masquerade hack.)

The general syntax for this macro is

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
    // Every domain, table, and property should be registered
    // before creating the Universe.
    MY_DOMAIN::register();
    my_table::register();

    // Every member of MY_DOMAIN is initialized at this point.
    // The universe owns a `RwLock` for each table & property.
    let universe = &Universe::new(&[MY_DOMAIN]);

    // Lock the table for writing.
    let mut my_table = my_table::write(universe);
    my_table.push(my_table::Row {
        my_int: 42,
    });
}
```

# Table `kind`s, Consistency, and Guarantees
The 'kind' of a table selects what functions are generated and what guarantees are upheld.

## `#[kind = "append"]`
This is the simplest kind.
Rows in an "append" table can not be removed.
Consistency is thus trivially guaranteed.

## `#[kind = "sorted"]`
Guarantees the table is sorted. You must implement `Ord` for `$table::RowRef`.
If you put `#[sort_key]` on a column, it will do this for you.
(The macro derives `Eq`, `PartialEq`, and `PartialOrd` on `$table::RowRef`, and those + `Ord` on `$table::Row`)

Rows can be added with `merge`, and removed with `retain`.

Sorted tables are good for [`joincore`].

## `#[kind = "consistent"]`
Rows in consistent tables can be used as *foreign keys* in other tables.
The main guarantee of the public table is that it is kept consistent with such tables:
the main row and references to it are deleted as a unit.

Since maintaining consistency requires locking other tables,
you must call `table.flush(universe, event)` instead of letting the table drop.

## `#[kind = "bag"]`
NYI. (Row order would be arbitrary and there would be no consistency guarantee.)

## `#[kind = "list"]`
NYI. (Row order would remain intact, but there would be no consistency guarantee.)

## `#[kind = "indirect"]`
NYI. (There would be a table of handles introducing a layer of indirection, but making it easy to implement certain guarantees.)

# Using the generated table

A lock on the table must be obtained using `$tablename::read(universe)`.

(FIXME: Link to `v11::example`)

# Table Attributes

This works like so:

```no_compile
table! {
    #[kind = "…"]
    #[table_attribute_1]
    #[table_attribute_2]
    [DOMAIN/table] {
        …
    }
}
```

## `#[row_id = "usize"]`
Sets what the (underlying) primitive is used for indexing the table. The default is `usize`.
This is useful when this table is going to have foreign keys pointing at it.

## `#[row_derive(Foo, Bar)]`
Puts `#[derive(Foo, Bar)]` on the generated `Row` and `RowRef` structs.

## `#[version = "0"]`
A version number for the table. The default is `0`. It is a `u32`.

# Column Attributes

```no_compile
table! {
    #[kind = "…"]
    pub [MY_DOMAIN/my_table] {
        #[column_attribute_1]
        #[column_attribute_2]
        my_int: [i32; VecCol<i32>],
    }
}
```

## `#[foreign]`
The row's element must be another table's RowId.
This generates a `struct track_$COL_events`, for which `Tracker` must be implemented, to react to structural events on the foreign table.

## `#[foreign_auto]`
This automatically implements `Tracker`. Rows corresponding to deleted foreign rows will be removed.
This requires `#[index]` or `#[sort_key]` on the local table.

## `#[index]`
Creates an index of the column, using a `BTreeMap`.
Indexed elements are immutable, and are duplicated.

## `#[sort_key]`
Use the element's comparision order to derive `Ord` for `RowRef`.

## `#[add_tracker = "expression"]`
Register a user tracker automatically when the table is initialized using the given expression.
For example, `#[add_tracker = "BirdWatch"]`.
You would then need to define `BirdWatch` and implement [`tracking::Tracker`] on it.

Can be repeated.
The trackers from `#[foreign]` and `#[foreign_auto]` take care of themselves;
using this on the trackers they define would duplicate it.

# Debugging Macro Output
If there is an error in the macro's output, it will give opaque errors and thus be impossible to debug... *UNTIL NOW*!
Simply define the `V11_MACRO_DUMP=*` environment variable before compilation,
and *ALL* `table!` macro output will be written to & loaded from a convenient file!!
You can also obtain the output of a specific table using its name, like `V11_MACRO_DUMP=heyo`!
`V11_MACRO_DUMP_DIR` can be used to write files to a specific directory!
If you don't specify this environmental variable, that's OKAY,
files are written to `target/v11_dump` by default!! It doesn't get ANY EASIER than HAYO-Corp!!!!


<small>*
Do not use if any separate tables have the same domain & name.
Such duplicate table names may result in explosions.
Do not compile multiple profiles with this feature enabled.
Simultaneous cargo/rustc invokations are unsupported, and may result in explosions.

HAYO-Corp is not liable for any loss of life, liberty, property, or data consistency due to misuse of product.
</small>

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
        #[allow(unused_imports)]
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
        #[allow(unused_imports)]
        pub mod $name {
            __v11_invoke_table! {
                __v11_internal_table!($(#[$meta])* [$domain/$name] $($args)*)
            }
        }
    };
}

