use std::io::Write;

use quote::{Ident, Tokens};
use syntex_syntax::print::pprust as pp;

use super::table::Table;

/// Convert a string into a quote `Ident`.
fn i<S: AsRef<str>>(s: S) -> Ident {
    Ident::new(s.as_ref())
}

/// Convert a `Vec` of strings into a vec of quote `Ident`s.
fn str2i(v: &Vec<String>) -> Vec<Ident> {
    v.iter().map(i).collect()
}

/// Possibly change `q` into an empty set of `Tokens`.
fn quote_if(b: bool, q: Tokens) -> Tokens {
    if b { q } else { quote! {} }
}

#[allow(non_snake_case)]
pub fn write_out<W: Write>(table: Table, mut out: W) -> ::std::io::Result<()> {
    /// Writes out one or zero of the branches.
    macro_rules! out {
        () => {
            { }
        };
        ($cond:expr => [$label:expr] { $($q:tt)* }; $($rest:tt)*) => {
            if $cond {
                out!(@quote $label; $($q)*);
            } else {
                out!($($rest)*);
            }
        };
        (let $pat:pat = $expr:expr => [$label:expr] { $($q:tt)* }; $($rest:tt)*) => {
            if let $pat = $expr {
                out!(@quote $label; $($q)*);
            } else {
                out!($($rest)*);
            }
        };
        ({ $($q:tt)* }) => {
            out!(@quote ""; $($q)*);
        };
        ([$label:expr] { $($q:tt)* }) => {
            out!(@quote $label; $($q)*);
        };
        ([$label:expr] { $($q:tt)* };) => {
            out!(@quote $label; $($q)*);
        };
        (@quote $label:expr; $($q:tt)*) => {
            let args = quote! { $($q)* };
            let buff = if $label.is_empty() {
                format!("{}\n", args)
            } else {
                format!("\n// {}\n{}\n\n", $label, args)
            };
            // Fix docs.
            let buff = buff.replace("#TABLE_NAME", &table.name);

            // It's very sweet that we don't have to pass this into the macro!
            // A benefit of defining the macro within the function.
            out.write(buff.as_bytes())?;
        }
    }

    // Info. It can get pretty long!
    writeln!(out, "// Table config:")?;
    for line in format!("{:#?}", table).split('\n') {
        writeln!(out, "//   {}", line)?;
    }

    // "name": ["element"; "col_type"],
    use ::table::Col;
    let COL_NAME_STR: &Vec<_> = &table.cols.iter().map(|x| pp::ident_to_string(x.name)).collect();
    let COL_ELEMENT_STR: &Vec<_> = &table.cols.iter().map(|x| pp::ty_to_string(&*x.element)).collect();
    let COL_TYPE_STR: &Vec<_> = &table.cols.iter()
        .map(|x| {
            let ct = pp::ty_to_string(&*x.colty);
            if x.indexed {
                // FIXME: More index types
                format!("Col<BTreeIndex<{}, Row>, Row>", ct)
            } else {
                format!("Col<{}, Row>", ct)
            }
        }).collect();
    let COL_ATTR: &Vec<Ident> = &table.cols.iter().map(|x: &Col| -> String {
        x.attrs.iter().map(pp::attr_to_string).map(|x| format!("{}\n", x)).collect()
    }).map(i).collect();

    // name: [element; col_type],
    let COL_NAME: &Vec<_> = &str2i(COL_NAME_STR);
    let COL_ELEMENT: &Vec<_> = &str2i(COL_ELEMENT_STR);
    let COL_TYPE: &Vec<_> = &str2i(COL_TYPE_STR);
    let COL0 = &COL_NAME[0];
    //let COL_COUNT: usize = table.cols.len();

    let COL_FORMAT: &Vec<String> = &table.cols.iter().map(|x| {
        // table.column: [element; column]
        format!("{}.{}: [{}; {}]",
                table.name,
                x.name,
                pp::ty_to_string(&*x.element),
                pp::ty_to_string(&*x.colty))
    }).collect();

    // Work around for things like #(#COL_NAME: row.#COL_NAME)* triggering a weird bug in `quote!`.
    let COL_NAME2 = COL_NAME;
    let COL_NAME3 = COL_NAME;
    let COL_NAME4 = COL_NAME;
    let COL_NAME5 = COL_NAME;
    let COL_NAME6 = COL_NAME;
    let COL_TYPE2 = COL_TYPE;

    let TABLE_NAME_STR = table.name.clone();
    #[allow(unused)]
    let TABLE_VERSION = table.version;
    let TABLE_DOMAIN = i(table.domain.clone());
    let TABLE_DOMAIN_STR = &table.domain;
    let GUARANTEES = {
        let CONSISTENT = table.consistent;
        let SORTED = table.sorted;
        let APPEND_ONLY = table.kind == Some(::table::TableKind::Append);
        quote! {
            Guarantee {
                consistent: #CONSISTENT,
                sorted: #SORTED,
                append_only: #APPEND_ONLY,
            }
        }
    };
    //: bool = table.sort_key.is_some();
    out! { ["Imports"] {
        // These imports are messy. It's complex because some of them are imported in multiple
        // ways. However, nobody cares.
        //

        #[allow(unused_imports)]
            extern crate v11;
            use self::v11::Universe;
            use self::v11::domain::DomainName;
            use self::v11::intern::{self, BiRef};
            use self::v11::tables::*;
            use self::v11::columns::*;
            use self::v11::index::{CheckedIter, Checkable};

            // This is a reasonable convenience for the user.
            use self::v11::storage::*;

            use self::v11::joincore::*;
            use self::v11::map_index::BTreeIndex;
            use self::v11::Action;
            use self::v11::tracking::{Tracker, GetParam, Select, SelectOwned, SelectRows, SelectAny, Flush};
            use self::v11::event::Event;
            use std::collections::VecDeque;
            use std::cmp::Ordering;
        use super::*;
    }}
    out! { ["Header info"] {
        pub const TABLE_NAME: TableName = TableName(#TABLE_NAME_STR);
        pub const TABLE_DOMAIN: DomainName = super::#TABLE_DOMAIN;
        pub const VERSION: u32 = #TABLE_VERSION;
        pub const GUARANTEES: Guarantee = #GUARANTEES;

        #[allow(non_upper_case_globals)]
        mod column_format {
            #(pub const #COL_NAME: &'static str = #COL_FORMAT;)*
        }
    }}

    let ROW_ID_TYPE = i(&table.row_id);
    out! { ["Indexing"] {
        /// The internal index type, which also limits the maximum number of rows.
        pub type RawType = #ROW_ID_TYPE;

        /// This is the type used to index into `#TABLE_NAME`'s columns.
        /// It is unique to the table.
        pub type RowId = GenericRowId<Row>;

        /// An index that is known to be valid for the lifetime of a read lock.
        pub type CheckIdRead<'u> = CheckedRowId<'u, Read<'u>>;
        /// An index that is known to be valid for the lifetime of a write lock.
        pub type CheckIdWrite<'u> = CheckedRowId<'u, Write<'u>>;

        /// This trait assists in converting between `RowId`, `CheckIdRead`, and `CheckIdWrite`.
        ///
        /// # Usage
        ///
        /// ```no_compile
        /// fn my_table_method<C: my_table::CheckId>(table: &my_table::Write, mine: C) {
        ///     let mine = mine.check(table);
        ///     // ...
        /// }
        /// ```
        pub trait CheckId: Checkable<Row = Row> {}
        impl CheckId for RowId {}
        impl<'u> CheckId for CheckIdRead<'u> {}
        impl<'u> CheckId for CheckIdWrite<'u> {}

        /// A reference to the first row. Is invalid if there is no rows.
        pub const FIRST: RowId = RowId {
            i: 0,
            table: ::std::marker::PhantomData,
        };

        /// An index value to be used for default values.
        /// Note that it may become valid if the table is full!
        pub const INVALID: RowId = RowId {
            i: ::std::usize::MAX as RawType,
            table: ::std::marker::PhantomData,
        };

        /// Creates an index into the `i`th row.
        pub fn at(i: #ROW_ID_TYPE) -> RowId { RowId::new(i) }
    }}

    let ROW_DERIVES: &Vec<_> = &table.row_derive.iter()
        .map(pp::meta_list_item_to_string)
        .map(i)
        .collect();

    let ROW_REF_DERIVES: Vec<_> = ROW_DERIVES.iter()
        .filter(|x| match x.as_ref() {
            "Clone" | "RustcEncodable" | "RustcDecodable" | "Serialize" | "Deserialize" => false,
            _ => true,
        }).collect();

    out! { ["The `Row` struct"] {
        use std::sync::RwLock;
        /**
         * A structure holding a copy of each column's data. This is used to pass entire rows around through methods;
         * the actual table is column-based, so eg `read.column[index]` is the standard method of accessing rows.
         * */
        #(#[derive(#ROW_DERIVES)])*
        pub struct Row {
            #(#COL_ATTR pub #COL_NAME: #COL_ELEMENT,)*
        }
        impl GetTableName for Row {
            type Idx = RawType;
            fn get_domain() -> DomainName { TABLE_DOMAIN }
            fn get_name() -> TableName { TABLE_NAME }
            fn get_guarantee() -> Guarantee { GUARANTEES }
            fn get_generic_table(universe: &Universe) -> &RwLock<GenericTable> {
                RowId::get_generic_table(universe)
            }
            fn new_generic_table() -> GenericTable { new_generic_table() }
        }

        /// A row of a reference to each element.
        #(#[derive(#ROW_REF_DERIVES)])*
        #[derive(Clone)]
        // Do we want RowRef to *always* be Copy? There could be a lot of rows!
        pub struct RowRef<'a> {
            #(#COL_ATTR pub #COL_NAME: &'a #COL_ELEMENT,)*
        }
        impl Row {
            /// Convert this `Row` into a `RowRef`.
            pub fn as_ref(&self) -> RowRef {
                RowRef {
                    #(#COL_NAME: &self.#COL_NAME2,)*
                }
            }
        }

        // FIXME: Implement `struct RowMut`, would need to respect EditA.
    }};
    out! { ["The `Table` struct"] {
        #[derive(Default)]
        pub struct Table {
            flush: Flush<Row>,
            free: FreeList<Row>,
        }
        impl TTable for Table {
            fn new() -> Self where Self: Sized { Default::default() }
            fn domain() -> DomainName where Self: Sized { TABLE_DOMAIN }
            fn name() -> TableName where Self: Sized { TABLE_NAME }
            fn guarantee() -> Guarantee where Self: Sized { GUARANTEES }
            // FIXME: Unused?

            fn prototype(&self) -> Box<TTable> { Box::new(Self::new()) }
            fn get_flush_ref(&self) -> &::std::any::Any { &self.flush }
            fn get_flush_mut(&mut self) -> &mut ::std::any::Any { &mut self.flush }

            fn remove_rows(&mut self, universe: &Universe, event: Event, rows: SelectAny) {
                Table::remove_rows(self, universe, event, rows);
            }

            fn serial_selection<'a>(&self, rows: &'a SelectAny) -> BoxedSerialize<'a> {
                Box::new(rows.as_ref().map(|rows| {
                    rows
                        .downcast::<RowId>()
                        .expect("selection element type mismatch")
                })) as BoxedSerialize
            }
        }
    }};
    out! {
        table.consistent => ["`Table` consistent"] {
            impl Table {
                fn remove_rows(&mut self, universe: &Universe, event: Event, rows: SelectAny) {
                    let mut table = write(universe);
                    match rows {
                        Select::These(rows) => {
                            if let Some(rows) = rows.downcast::<RowId>() {
                                for row in rows {
                                    table.delete(*row);
                                }
                            } else {
                                panic!("wrong rows type");
                            }
                        },
                        Select::All => table.clear(),
                    }
                    table.flush(universe, event);
                }
            }
        };
        // FIXME: We can remove if we're `#[kind = "sorted"]`, but the rows'll have to be sorted...
        true => ["`Table` inconsistent"] {
            impl Table {
                fn remove_rows(&mut self, _: &Universe, _: Event, _: SelectAny) {
                    panic!("Rows can not be removed on this table.");
                }
            }
        };
    }

    out! { table.derive.clone => ["RowRef IntoOwned"] {
        impl<'a> RowRef<'a> {
            pub fn to_owned(&self) -> Row {
                Row {
                    #(#COL_NAME: self.#COL_NAME2.clone(),)*
                }
            }
        }
    };};

    let COL_MUT: &Vec<_> = &table.cols.iter()
        .map(|x| if x.indexed { "EditA" } else { "MutA" })
        .map(i)
        .collect();

    let LOCKED_TABLE_DELETED_ROW = quote_if(table.consistent, quote! {
        fn is_deleted(&self, idx: GenericRowId<Row>) -> bool {
            self._table.free.get(&idx).is_some()
        }
    });
    out! { ["Table locks"] {
        /**
         * The table, locked for reading.
         * */
        pub struct Read<'u> {
            _lock: BiRef<::std::sync::RwLockReadGuard<'u, GenericTable>, &'u GenericTable, GenericTable>,
            _table: &'u Table,
            #(pub #COL_NAME: RefA<'u, #COL_TYPE>,)*
        }

        /**
         * The table, locked for writing.
         * */
        pub struct Write<'u> {
            _universe: &'u Universe, // FIXME: Remove 'verse from Write's methods.
            _lock: ::std::sync::RwLockWriteGuard<'u, GenericTable>,
            _table: &'u mut Table,
            // '#COL_MUT' is either MutA or EditA
            #(pub #COL_NAME: #COL_MUT<'u, #COL_TYPE>,)*
        }

        /// The table, borrowed from a `Write` lock, that forbids structural changes.
        pub struct Edit<'u, 'w> where 'u: 'w {
            _inner: &'w Write<'u>,
            #(pub #COL_NAME: &'w mut #COL_MUT<'u, #COL_TYPE>,)*
        }
        impl<'u> Write<'u> {
            /// Divides this lock into two parts.
            ///
            /// 1. An `Edit`, which allows modifying rows, but not making structural modifications
            ///    to the table,
            /// 2. An `EditIter`, which allows iteration over the table, while skipping deleted
            ///    rows.
            pub fn editing<'w>(&'w mut self) -> (Edit<'u, 'w>, EditIter<'w, Row>)
            where 'u: 'w
            {
                unsafe {
                    // This is conceptually/morally equivalent to `slice::split_at_mut`.
                    // It's like splitting `Write` into `(&Structural, &mut Edit)`.
                    // See the `Deref` implementation.
                    // FIXME: Is it UB to have `&T` and `&mut T.1`, even if we make guarantees?
                    use std::mem;
                    let me1: &Self = mem::transmute(self as *const Self);
                    let me2: &mut Self = mem::transmute(self as *mut Self);
                    let me3 = self;
                    (Edit {
                        _inner: me1,
                        #(#COL_NAME: &mut me2.#COL_NAME2,)*
                    }, EditIter::new(me3.row_range(), me3._table.free.keys()))
                }
            }
        }
        // By good fortune this is safe. Implementing DerefMut would *not* be safe.
        // Suppose we use Deref to call `Write::get_row_ref(edit)`. Can we mutably alias
        // the returned reference with `edit.col`? No, because RowRef is borrowing edit.
        // Because the `Write` is immutable, no structural changes can modify
        // `EditIter.deleted`. It would be unsafe to implement `DerefMut`.
        impl<'u, 'w> ::std::ops::Deref for Edit<'u, 'w> where 'u: 'w {
            type Target = Write<'u>;
            fn deref(&self) -> &Write<'u> {
                self._inner
            }
        }

        impl<'u> LockedTable for Read<'u> {
            type Row = Row;
            fn len(&self) -> usize { self.len() }
            #LOCKED_TABLE_DELETED_ROW
        }
        impl<'u> LockedTable for Write<'u> {
            type Row = Row;
            fn len(&self) -> usize { self.len() }
            #LOCKED_TABLE_DELETED_ROW
        }
    }};


    let RW_FUNCTIONS_CONSISTENT = quote! {
        /** Iterate over a range of rows. (R/W) */
        pub fn range(&self, range: RowRange<RowId>) -> ConsistentIter<Self> {
            let checked_iter = CheckedIter::from(self, range);
            ConsistentIter::new(checked_iter, &self._table.free)
        }

        /** Returns true if `i` is a valid RowId. */
        pub fn contains(&self, index: RowId) -> bool {
            index.to_usize() < self.len() && !self._table.free.contains_key(&index)
        }
    };
    let DUMP = quote_if(table.derive.clone, quote! {
        /** Allocates a Vec filled with every Row in the table. (R/W) */
        // we exclude consistent, because deleted rows shouldn't be included, but then if you
        // reconstitute, indexes would be wrong.
        pub fn dump(&self) -> Vec<Row> {
            let mut ret = Vec::with_capacity(self.len());
            for i in self.iter() {
                ret.push(self.get_row(i));
            }
            ret
        }
    });
    out! {
        !table.consistent => ["inconsistent iterators"] {
            impl<'u> Read<'u> {
                /// Returns a pre-checking iterator over each row in the table.
                pub fn iter(&self) -> CheckedIter<Self> {
                    self.range(self.row_range())
                }
            }
            impl<'u> Write<'u> {
                /// Returns a pre-checking iterator over each row in the table.
                pub fn iter(&self) -> CheckedIter<Self> {
                    self.range(self.row_range())
                }

                /// Returns an iterator over each row in the table. The `RowId`s are not
                /// pre-checked; you should consider calling `row_id.check($table)`, particularly
                /// if you will be indexing many columns.
                pub fn iter_mut(&mut self) -> UncheckedIter<Row> {
                    // Well, the `&mut self` isn't actually necessary.
                    self.row_range().iter_slow()
                }
            }
        };
        ["consistent iterators"] {
            impl<'u> Read<'u> {
                /// Iterate over every non-deleted row.
                ///
                /// (Use `Write::editing` to iterate over a `Write` lock.)
                pub fn iter(&self) -> ConsistentIter<Self> {
                    self.range(self.row_range())
                }
            }
            impl<'u> Write<'u> {
                /// Iterate over every non-deleted row. Note that this is an immutable iterator;
                /// use `editing` to get at an editable iterator.
                pub fn iter(&self) -> ConsistentIter<Self> {
                    self.range(self.row_range())
                }

                /// Iterate over *all* rows, including deleted ones.
                /// See: `$table.contains()`
                pub fn iter_del(&self) -> UncheckedIter<Row> {
                    self.row_range().iter_slow()
                }
            }
        };
    };
    let RW_FUNCTIONS_INCONSISTENT = quote! {
        /** Iterate over a range of rows. (R/W) */
        pub fn range(&self, range: RowRange<RowId>) -> CheckedIter<Self> {
            CheckedIter::from(self, range)
        }

        /** Returns true if `i` is a valid RowId. */
        pub fn contains(&self, index: RowId) -> bool {
            index.to_usize() < self.len()
        }

        #DUMP
    };
    let GET_ROW = quote_if(table.derive.clone, quote! {
        /** Retrieves a structure containing a clone of the value in each column. (R/W) */
        pub fn get_row<R: Checkable<Row=Row>>(&self, index: R) -> Row where Row: Clone {
            let index = index.check(self);
            self.get_row_raw(index)
        }

        fn get_row_raw(&self, index: CheckedRowId<Self>) -> Row where Row: Clone {
            Row {
                #(#COL_NAME: self.#COL_NAME2[index].clone(),)*
            }
        }
    });
    let RW_FUNCTIONS_BOTH = quote! {
        /** Returns the number of rows in the table. (R/W) */
        // And assumes that the columns are all the same length.
        // But there shouldn't be any way to break that invariant.
        pub fn len(&self) -> usize {
            self.#COL0.deref().inner().len()
        }

        /// Equivalent to `0..len()`. (R/W)
        ///
        /// Be careful calling this on consistent tables; it may include deleted rows.
        /// See `iter`.
        fn row_range(&self) -> RowRange<RowId> {
            (RowId::new(0)..RowId::from_usize(self.len())).into()
        }

        #GET_ROW

        /** Retrieves a structure containing a reference to each value in each column. (R/W) */
        pub fn get_row_ref<R: Checkable<Row=Row>>(&self, index: R) -> RowRef {
            let index = index.check(self);

            RowRef {
                #(#COL_NAME: &self.#COL_NAME2[index],)*
            }
        }

        /// Gets the last `RowId`.
        pub fn last(&self) -> Option<RowId> {
            // FIXME: Add a consistent version?
            let r = self.len();
            if r == 0 {
                None
            } else {
                Some(RowId::from_usize(r - 1))
            }
        }

        /// Explicitly drop the lock. (R/W)
        pub fn close(self) { /* You are not expected to understand this. */ }
    };
    let RW_FUNCTIONS = if table.consistent { RW_FUNCTIONS_CONSISTENT } else { RW_FUNCTIONS_INCONSISTENT };
    out! { ["methods common to both Read and Write"] {
        // We're only repeating ourselves twice here.

        impl<'u> Read<'u> {
            #RW_FUNCTIONS
            #RW_FUNCTIONS_BOTH
        }
        impl<'u> Write<'u> {
            #RW_FUNCTIONS
            #RW_FUNCTIONS_BOTH

            // FIXME: add `fn iter()` that returns an iterator yielding MaybeDeleted things.
        }
    }};

    out! {
        table.consistent => ["Change tracking"] {
            impl<'a> Write<'a> {
                /// Propagate all changes
                pub fn flush(self, universe: &Universe, event: Event) {
                    if !self._table.flush.need_flush() { return; }
                    let mut flush = self._table.flush.extract();
                    ::std::mem::drop(self);
                    flush.flush(universe, event);
                    write(universe)._table.flush.restore(flush);
                }

                /// Flush table without releasing the lock. This will of course cause a deadlock if
                /// the table has trackers that need to look at values.
                pub fn live_flush(&mut self, universe: &Universe, event: Event) {
                    if !self._table.flush.need_flush() { return; }
                    self._table.flush.flush(universe, event);
                }

                pub fn delete<I: CheckId>(&mut self, row: I) {
                    unsafe {
                        let i = row.check(self).uncheck();
                        self.delete_raw(i.to_usize());
                        self.event_del(i);
                        self._table.free.insert(i, ());
                    }
                }

                unsafe fn delete_raw(&mut self, i: usize) {
                    #(
                        self.#COL_NAME.deref_mut().inner_mut().deleted(i);
                    )*
                }

                /*
                /// Try to remove an instance of your tracker.
                pub fn remove_tracker<T: Tracker<Table=Row>>(&mut self) -> Option<Box<Tracker<Table=Row>>> {
                    self._table.flush.remove_tracker::<T>()
                }
                */
                /// This method is here as a convenience for macros.
                pub fn flush_or_close(self, universe: &Universe, event: Event) { self.flush(universe, event) }

                /// Mark a row. Calling `flush` will then propagate the selection.
                pub fn select(&mut self, row: RowId) {
                    self._table.flush.select(row);
                }

                /// Mark every row. Calling `flush` will then propagate the selection.
                pub fn select_all(&mut self) {
                    self._table.flush.select_all();
                }
            }
            impl<'a> Read<'a> {
                /// Select every row. (Immutable version; also defined for [`Write`].)
                pub fn select_all(
                    &self,
                    universe: &Universe,
                    event: Event,
                ) {
                    self._table.flush.flush_all(universe, event)
                }

                pub fn select_rows<I>(
                    &self,
                    universe: &Universe,
                    event: Event,
                    selection_sorted: bool,
                    selection: I,
                )
                where
                    I: Iterator<Item=RowId>,
                {
                    self._table.flush.flush_selection(
                        universe,
                        event,
                        selection_sorted,
                        selection,
                    )
                }
            }

            /// Makes sure the flush requirement has been acknowledged
            impl<'a> Drop for Write<'a> {
                fn drop(&mut self) {
                    if self._table.flush.need_flush() {
                        panic!("Changes to {} were not flushed", TABLE_NAME);
                    }
                }
            }
        };
        ["fake flush"] {
            impl<'u> Write<'u> {
                /// This method is here as a convenience for macros.
                pub fn flush_or_close(self, _: &Universe, _: Event) {}

                // "shouldn't" get called; could happen if the table kind changes between
                // serializations. This is a stub.
                unsafe fn delete_raw(&mut self, _i: usize) {
                    panic!("Unexpected call to delete_raw");
                }
            }
        };
    }

    // 'ifc' = "indexed foreign column"
    let ifcs = || table.cols.iter().filter(|x| x.indexed && x.foreign);
    let IFC: Vec<_> = ifcs()
        .map(|x| i(pp::ident_to_string(x.name)))
        .collect();
    let TRACK_IFC_REMOVAL: Vec<_> = ifcs()
        .map(|x| i(format!("track_{}_removal", x.name)))
        .collect();
    let IFC_ELEMENT: Vec<_> = ifcs()
        .map(|x| i(pp::ty_to_string(&*x.element)))
        .collect();

    let sorted_foreign = || table.cols.iter().filter(|x| Some(x.name) == table.sort_key && x.foreign);
    let TRACKED_SORTED_COL: &Vec<_> = &sorted_foreign()
        .map(|x| i(pp::ident_to_string(x.name)))
        .collect();
    let TRACK_SORTED_COL_EVENTS: &Vec<_> = &sorted_foreign()
        .map(|x| i(format!("track_sorted_{}_removal", x.name)))
        .collect();
    let TRACK_SORTED_COL_ELEMENT: &Vec<_> = &sorted_foreign()
        .map(|x| i(pp::ty_to_string(&*x.element)))
        .collect();
    out! { ["foreign row removal"] {
        impl<'u> Write<'u> {
            #(
                /// `deleted` is a list of removed foreign keys.
                pub fn #TRACK_IFC_REMOVAL(&mut self, selected: Select<&[#IFC_ELEMENT]>) {
                    match selected {
                        Select::All => self.clear(),
                        Select::These(deleted) => {
                            for deleted_foreign in deleted {
                                // It'd be nicer to keep the iterator around, but we immediately
                                // invalidate it. We could collect it into a Vec?
                                while let Some(kill) = self.#IFC.deref().inner().find(*deleted_foreign).next() {
                                    // FIXME: Add a 'Sorted' wrapping TCol that exposes find() using binary search.
                                    self.delete(kill);
                                    if cfg!(test) && self.contains(kill) {
                                        panic!("Deletion failed");
                                    }
                                }
                            }
                        },
                    }
                }
            )*
            #(
                /// This is a table sorted by a foreign key. This function removes all the keys
                /// listed in `remove`, which must also be sorted.
                pub fn #TRACK_SORTED_COL_EVENTS(&mut self, select: Select<&[#TRACK_SORTED_COL_ELEMENT]>) {
                    match select {
                        Select::All => self.clear(),
                        Select::These(remove) => {
                            let mut core = JoinCore::new(remove.iter().map(|x| *x));
                            self.merge0(move |me, rowid| {
                                use std::iter::empty;
                                let foreign = me.#TRACKED_SORTED_COL[rowid];
                                match core.cmp(&foreign) {
                                    Join::Match(_) => Action::Continue { remove: true, add: empty() },
                                    Join::Next => Action::Continue { remove: false, add: empty() },
                                    Join::Stop => Action::Break,
                                }
                            });
                        },
                    }
                }
            )*
        }
    }};

    let mut FOREIGN_NAME_NONCE = Vec::new();
    let mut FOREIGN_LOCAL_COL = Vec::new();
    let mut FOREIGN_ELEMENTS = Vec::new();
    for col in &table.cols {
        if !col.foreign_auto { continue; }
        let TRACK_EVENTS = i(format!("track_{}_events", col.name));
        let TRACK_REMOVAL = i(if Some(col.name) == table.sort_key {
            format!("track_sorted_{}_removal", col.name)
        } else if col.indexed {
            format!("track_{}_removal", col.name)
        } else {
            panic!("`#[foreign_auto]` can only be used on columns with `#[index]` or `#[sort_key]`.");
        });
        let FOREIGN_ELEMENT = i(pp::ty_to_string(&*col.element));
        out! { ["foreign_auto"] {
            impl Tracker for #TRACK_EVENTS {
                // #FOREIGN_ELEMENT is a GenericRowId<TableRow>.
                // We need to get at the TableRow...
                type Foreign = <#FOREIGN_ELEMENT as GetParam>::T;

                fn consider(&self, event: Event) -> bool { event.is_removal }
                fn sort(&self) -> bool { GUARANTEES.sorted }

                fn handle(&self, universe: &Universe, event: Event, rows: SelectRows<Self::Foreign>) {
                    let mut table = write(universe);
                    table.#TRACK_REMOVAL(rows);
                    table.flush_or_close(universe, event);
                }
            }
        }};
        FOREIGN_NAME_NONCE.push(i(format!("_foreign_{}", col.name)));
        FOREIGN_LOCAL_COL.push(i(format!("{}", col.name)));
        FOREIGN_ELEMENTS.push(FOREIGN_ELEMENT);
    }
    let FOREIGN_NAME_NONCE = &FOREIGN_NAME_NONCE;
    let FOREIGN_LOCAL_COL = &FOREIGN_LOCAL_COL;
    let FOREIGN_LOCAL_COL2 = FOREIGN_LOCAL_COL;
    let FOREIGN_LOCAL_COL3 = FOREIGN_LOCAL_COL;

    out! {
        table.consistent => ["Extra drops"] {
            /// Prevent moving out columns to improve `RefA` safety.
            impl<'u> Drop for Read<'u> {
                fn drop(&mut self) {}
            }
        };
        ["Extra drops"] {
            /// Prevent moving out columns to improve `RefA` safety.
            impl<'u> Drop for Read<'u> {
                fn drop(&mut self) {}
            }

            /// Prevent moving out columns to improve `MutA` safety.
            impl<'u> Drop for Write<'u> {
                fn drop(&mut self) {}
            }
        };
    }

    out! {
        table.consistent => ["event logging for consistent tables"] {
            impl<'u> Write<'u> {
                #[inline] fn event_cleared(&mut self) { self._table.flush.select_all() }
                #[inline] fn event_add(&mut self, i: RowId) { self._table.flush.select(i) }
                #[inline] fn event_del(&mut self, i: RowId) { self._table.flush.select(i) }
                #[inline] fn event_add_reserve(&mut self, n: usize) { self._table.flush.reserve(n) }
                #[inline] fn event_del_reserve(&mut self, n: usize) { self._table.flush.reserve(n) }
            }
        };
        ["event ignoring for inconsistent_columns tables"] {
            impl<'u> Write<'u> {
                #[inline] fn event_cleared(&mut self) {}
                #[inline] fn event_add(&mut self, _: RowId) {}
                #[inline] fn event_del(&mut self, _: RowId) {}
                #[inline] fn event_add_reserve(&mut self, _: usize) {}
                #[inline] fn event_del_reserve(&mut self, _: usize) {}
            }
        };
        // FIXME: event_del_reserve is inacessible!
    }

    out! { ["mut methods safe for all guarantees"] {
        impl<'u> Write<'u> {
            /** Prepare the table for insertion of a specific amount of data. `self.len()` is
             * unchanged. */
            pub fn reserve(&mut self, n: usize) {
                #(self.#COL_NAME.deref_mut().inner_mut().reserve(n);)*
                self.event_add_reserve(n);
            }

            /// Removes every row from the table.
            pub fn clear(&mut self) {
                #(self.#COL_NAME.deref_mut().inner_mut().clear();)*
                self.event_cleared();
            }

            /// Not really 'safe', but it's private.
            /// Add a Row to the end of the table, without checking the free-list.
            #[inline]
            fn push_end_unchecked(&mut self, row: Row) -> RowId {
                let rowid = self.push_only_unchecked(row);
                self.event_add(rowid);
                rowid
            }

            #[inline]
            fn push_only_unchecked(&mut self, row: Row) -> RowId {
                #(self.#COL_NAME.deref_mut().inner_mut().push(row.#COL_NAME2);)*
                self.last().unwrap()
            }

        }
    }};

    out! {
        table.sorted => ["derive Ord for Row from RowRef"] {
            impl PartialOrd for Row {
                fn partial_cmp(&self, rhs: &Row) -> Option<Ordering> {
                    Some(self.cmp(rhs))
                }
            }

            impl Ord for Row {
                fn cmp(&self, rhs: &Row) -> Ordering {
                    self.as_ref().cmp(&rhs.as_ref())
                }
            }

            impl PartialEq for Row {
                fn eq(&self, rhs: &Row) -> bool {
                    self.cmp(rhs) == Ordering::Equal
                }
            }

            impl Eq for Row {}

            impl<'a> PartialOrd for RowRef<'a> {
                fn partial_cmp(&self, rhs: &RowRef) -> Option<Ordering> {
                    Some(self.cmp(rhs))
                }
            }

            impl<'a> PartialEq for RowRef<'a> {
                fn eq(&self, rhs: &RowRef) -> bool {
                    self.cmp(rhs) == Ordering::Equal
                }
            }

            impl<'a> Eq for RowRef<'a> {}
        };
    };
    if let Some(sort_key) = table.sort_key {
        let SORT_KEY = i(pp::ident_to_string(sort_key));
        out! { [""] {
            impl<'a> Ord for RowRef<'a> {
                fn cmp(&self, rhs: &Self) -> Ordering {
                    self.#SORT_KEY.cmp(&rhs.#SORT_KEY)
                }
            }
        }};
    }

    out! {
        table.sorted || !table.immutable => ["swapping"] {
            // Making this public would break many guarantees!
            impl<'u> Write<'u> {
                #[inline]
                fn swap_out_row(&mut self, i: RowId, row: &mut Row) {
                    unsafe {
                        let i = i.check(self).to_usize();
                        #(self.#COL_NAME.deref_mut().inner_mut().unchecked_swap_out(i, &mut row.#COL_NAME2);)*
                    }
                }

                #[inline]
                fn swap(&mut self, a: RowId, b: RowId) {
                    unsafe {
                        let a = a.check(self).to_usize();
                        let b = b.check(self).to_usize();
                        #(self.#COL_NAME.deref_mut().inner_mut().unchecked_swap(a, b);)*
                    }
                }

                #[inline]
                fn truncate(&mut self, new_len: usize) {
                    #(self.#COL_NAME.deref_mut().inner_mut().truncate(new_len);)*
                }
            }
        };
    };

    out! { !table.immutable && table.derive.clone && !table.consistent => ["merge functions"] {
        impl<'u> Write<'u> {
            /// Remove all rows for which the predicate returns `false`.
            pub fn retain<F: FnMut(&Self, RowId) -> bool>(&mut self, mut f: F) {
                // FIXME: Retain, but w/ early exit.
                self.merge0(|me, rowid| {
                    Action::Continue {
                        remove: !f(me, rowid),
                        add: ::std::iter::empty(),
                    }
                })
            }

            // We use a physically-inspired bulging rug algorithm.
            // There are four actions that happen here.
            // The first two happen while iterating over the table.
            // 1: on each row we push_back a row to keep, and an iterator to run
            // 2: we pop off the front of the rug to fill in the gaps created by deletions.
            // 3: after the iteration, there may be rows to trim.
            // 4: we push out the contents of the rug.
            fn merge0<IT, F>(&mut self, mut f: F)
            where
                IT: IntoIterator<Item = Row>,
                F: FnMut(&Self, RowId) -> Action<IT>,
            {
                // It'd be nice to use a type alias here...
                let mut rug: VecDeque<Result<Row, IT::IntoIter>> = VecDeque::new();

                // Try to remove a single row from the rug.
                let pull_rug = |rug: &mut VecDeque<Result<Row, IT::IntoIter>>| {
                    while let Some(rug_next) = rug.pop_front() {
                        match rug_next {
                            Ok(rug_row) => return Some(rug_row),
                            Err(mut iter) => {
                                if let Some(next) = iter.next() {
                                    rug.push_front(Err(iter));
                                    return Some(next);
                                }
                            }
                        }
                    }
                    None
                };
                // entries in `rug_front..rug_back` are uninitialized.
                let mut rug_front = FIRST;
                let mut rug_back = FIRST;
                let mut stopped = false;
                while rug_back.to_usize() < self.len() {
                    let action = if stopped { Action::Break } else { f(self, rug_back) };
                    match action {
                        Action::Continue { remove, add } => {
                            if !remove {
                                rug.push_back(Ok(self.get_row(rug_back)));
                            }
                            rug_back = rug_back.next();
                            rug.push_back(Err(add.into_iter()));
                        }
                        Action::Break => {
                            stopped = true;
                            if rug_front == rug_back && rug.is_empty() {
                                // We were a no-op
                                return;
                            }
                            // same as 'remove: false, add: None'
                            rug.push_back(Ok(self.get_row(rug_back)));
                            rug_back = rug_back.next();
                        }
                    }
                    while rug_front < rug_back {
                        if let Some(mut rug_row) = pull_rug(&mut rug) {
                            self.swap_out_row(rug_front, &mut rug_row);
                            rug_front = rug_front.next();
                        } else {
                            break;
                        }
                    }
                }
                self.truncate(rug_front.to_usize());
                while let Some(row) = pull_rug(&mut rug) {
                    self.push_end_unchecked(row);
                }
            }
        }
    };}
    out! { !table.immutable && table.derive.clone && !table.sorted && !table.consistent => ["visit"] {
        impl<'u> Write<'u> {
            pub fn visit<IT, F>(&mut self, f: F)
            where
                IT: IntoIterator<Item=Row>,
                F: FnMut(&Self, RowId) -> Action<IT>
            {
                self.merge0(f)
            }
        }
    };}
    out! {
        !table.immutable && table.sorted && !table.consistent => ["row pushing for sorted tables"] {
            impl<'u> Write<'u> {
                /// Merge in a sorted (or sortable) Iterator of `Row`s.
                pub fn merge<IT, I>(&mut self, rows: I)
                where
                    IT: Iterator<Item=Row>,
                    I: Into<AssertSorted<IT>>,
                {
                    self.merge_logged(rows, |_, _| ());
                }

                /// Merge in a Row, and return its RowId.
                /// This is an O(n) operation; so calling this in a loop will be O(n²).
                /// (The obnoxiously long name is to dissuade you from doing this.)
                pub fn merge_in_a_single_row(&mut self, row: Row) -> RowId {
                    let mut got = None;
                    self.merge_logged(Some(row), |_self, id| {
                        if got.is_none() {
                            got = Some(id);
                        } else {
                            panic!("same row merged twice");
                        }
                    });
                    got.expect("row not merged")
                }

                /// Merge in a sorted iterator of `Row`s.
                /// `log` will be called with the new RowId of each row.
                ///
                /// # Caveats
                /// The table will not be flushed before calls to `log`; if this is a problem you
                /// should collect the rows, call flush, and then act.
                pub fn merge_logged<IT, I, L>(&mut self, rows: I, mut log: L)
                where
                    IT: Iterator<Item=Row>,
                    I: Into<AssertSorted<IT>>,
                    L: FnMut(&Self, RowId),
                {
                    use std::iter::Peekable;

                    // This algorithm has three stages.
                    // NB: You'll probably want to simulate this with legos.
                    let mut cursor = FIRST;
                    let mut iter: Peekable<IT> = rows.into().into_iter().peekable();
                    let mut rug = VecDeque::<Row>::new();
                    {
                        // Stage 1
                        // Figure out where we need to start inserting from the iterator.
                        let first = if let Some(first) = iter.peek() {
                            first.as_ref()
                        } else {
                            return; // do-nothing
                        };
                        while cursor.to_usize() < self.len() && self.get_row_ref(cursor) < first {
                            cursor = cursor.next();
                        }
                    }

                    // fn for choosing the lower
                    enum Side { Rug, Iter }
                    let choose = |rug: &VecDeque<Row>, iter: &mut Peekable<IT>| -> Option<Side> {
                        let rug = rug.front().map(Row::as_ref);
                        let iter = iter.peek().map(Row::as_ref);
                        // #[allow(formatting)] // table makes logic obvious
                        Some(match (rug, iter) {
                            (None, None)                 => return None,
                            (None, Some(_))              => Side::Iter,
                            (Some(_), None)              => Side::Rug,
                            (Some(ref r), Some(ref i)) if r <= i => Side::Rug, // '<=', not '<', for sort stability
                            (Some(_), Some(_))           => Side::Iter,
                        })
                    };


                    let rest: RowRange<RowId> = (cursor..RowId::from_usize(self.len())).into();
                    for cursor in rest.iter_slow() {
                        // Stage 2
                        // pop lowest (iter, rug_front) -> bump to table -> bump to rug's back

                        let side = choose(&rug, &mut iter).expect("rug & iter ran out before end of table!?");
                        let mut val = if let Side::Rug = side {
                            rug.pop_front()
                        } else {
                            iter.next()
                        }.unwrap();
                        self.swap_out_row(cursor, &mut val);
                        if let Side::Iter = side {
                            log(self, cursor);
                        }
                        rug.push_back(val);
                    }
                    {
                        // Stage 3
                        // We're at the end of the table; flush the rest.
                        self.reserve(iter.size_hint().0 + rug.len());
                        while let Some(side) = choose(&rug, &mut iter) {
                            let val = match side {
                                Side::Rug => rug.pop_front(),
                                Side::Iter => iter.next(),
                            }.unwrap();
                            let cursor = self.push_end_unchecked(val);
                            if let Side::Iter = side {
                                log(self, cursor);
                            }
                        }
                    }
                }
            }
        };
        !table.sorted => ["row pushing for unsorted tables"] {
            impl<'u> Write<'u> {
                /// Populate the table with data from the provided iterator.
                pub fn push_all<I: ::std::iter::Iterator<Item=Row>>(&mut self, data: I) {
                    self.reserve(data.size_hint().0);
                    for row in data {
                        self.push(row);
                    }
                }

                /// Appends a single Row to the table.
                /// Returns its RowId. This is not necessarily at the end of the table!
                // In retrospect 'push' might have been a poor name.
                #[inline]
                pub fn push(&mut self, mut row: Row) -> RowId {
                    let expect = if cfg!(test) {
                        Some(self.next_pushed())
                    } else {
                        None
                    };
                    let next = self._table.free.keys().next().cloned();
                    let i = if let Some(old) = next {
                        self._table.free.remove(&old);
                        self.event_add(old);
                        // This is a very simple implementation!
                        self.swap_out_row(old, &mut row);
                        old
                    } else {
                        self.push_end_unchecked(row)
                    };
                    if cfg!(test) {
                        assert_eq!(Some(i), expect);
                    }
                    // It's not a checked index. I think it likely that you'll generally want an
                    // unchecked index when using this, for a foreign key.
                    i
                }

                /// Returns the RowId of the next row that would be inserted.
                pub fn next_pushed(&self) -> RowId {
                    self._table.free
                        .keys()
                        .next()
                        .cloned()
                        .unwrap_or(RowId::from_usize(self.len()))
                }

                /// Push an 'array' of values. The return value is a contiguous range.
                pub fn push_array<I>(&mut self, mut i: I) -> RowRange<RowId>
                where I: ExactSizeIterator<Item=Row>
                {
                    // This implementation doesn't need ExactSizeIterator, but future configurations
                    // using FreeList might require it.
                    let start = if let Some(row) = i.next() {
                        self.push_end_unchecked(row)
                    } else {
                        return RowRange::empty();
                    };
                    let mut end = start;
                    for row in i {
                        end = self.push_end_unchecked(row);
                    }
                    RowRange {
                        start,
                        end: end.next(),
                    }
                }
            }
        };
    }

    out! { ["Lock & Load"] {

        use std::mem::transmute;
        use std::sync::{RwLockReadGuard, RwLockWriteGuard, LockResult, TryLockResult};

        fn convert_read_guard(_lock: RwLockReadGuard<GenericTable>) -> Read {
            #(let #COL_NAME = {
                let got = _lock.get_column::<#COL_TYPE2>(#COL_NAME_STR, column_format::#COL_NAME2);
                unsafe {
                    RefA::new(transmute(got)) // ...YIKES!
                    // So, the struct returned has a _lock with lifetime 'u,
                    // but we need the columns to have a lifetime of the lock.
                    // Using RefA (or MutA) limits the column's lifetime to that of the struct.
                    //
                    // So if a column outlives _lock, there'll be trouble. How can this happen, and
                    // how is this prevented?
                    // 1. _lock is dropped: _lock is private.
                    // 2. column moved out of Read: Read implements Drop, preventing this.
                    // 3. column is swapped out of Read: this is a safety hole, but you're
                    //    REALLY working for trouble if you do this...
                }
                // If mem::swap becomes a problem, we could switch to OwningRef<Rc<RWLGuard>, &column>.
                // This does require heap allocation tho...
            };)*
            let _table = {
                let _table: &Table = _lock.table.downcast_ref::<Table>().expect("Table downcast failed");
                // FIXME: Audit this unsafety. This might be easy to another thing? Or
                // GenericTable'll fall out somehow. Maybe they could co-own?
                unsafe { transmute(_table) }
            };
            Read {
                _lock: BiRef::Left(_lock),
                _table,
                #( #COL_NAME3, )*
            }
        }

        /// Locks the table for reading.
        // We're too cool to be callling unwrap() all over the place.
        pub fn read(universe: &Universe) -> Read {
            read_result(universe).unwrap()
        }

        /// This is equivalent to `RwLock::read`.
        pub fn read_result<'u>(universe: &'u Universe) -> LockResult<Read<'u>> {
            let table = RowId::get_generic_table(universe).read();
            intern::wrangle_lock::map_result(table, convert_read_guard)
        }

        pub fn try_read<'u>(universe: &'u Universe) -> TryLockResult<Read<'u>> {
            let table = RowId::get_generic_table(universe).try_read();
            intern::wrangle_lock::map_try_result(table, convert_read_guard)
        }



        fn convert_write_guard<'u>(_universe: &'u Universe, mut _lock: RwLockWriteGuard<'u, GenericTable>) -> Write<'u> {
            #(let #COL_NAME = {
                let got = _lock.get_column_mut::<#COL_TYPE2>(#COL_NAME_STR, column_format::#COL_NAME2);
                unsafe {
                    #COL_MUT::new(transmute(got))
                    // See comment about transmute in `convert_read_guard()`.
                    // FIXME: Actually the comment should go on this one, since mut is harder.
                }
            };)*
            let _table = {
                let _table: &mut Table = _lock.table.downcast_mut::<Table>().expect("Table downcast failed");
                // See comment about transmute in `convert_read_guard()`.
                unsafe { transmute(_table) }
            };
            Write {
                _universe,
                _lock,
                _table,
                #( #COL_NAME3: #COL_NAME4, )*
            }
        }

        /// Locks the table for writing.
        pub fn write<'u>(universe: &'u Universe) -> Write<'u> {
            write_result(universe).unwrap()
        }

        pub fn write_result<'u>(universe: &'u Universe) -> LockResult<Write<'u>> {
            // FIXME: err, table is a Result, maybe we don't need wrangle_lock?
            let table = RowId::get_generic_table(universe).write();
            intern::wrangle_lock::map_result(table, |l| convert_write_guard(universe, l))
        }

        pub fn try_write<'u>(universe: &'u Universe) -> TryLockResult<Write<'u>> {
            let table = RowId::get_generic_table(universe).try_write();
            intern::wrangle_lock::map_try_result(table, |l| convert_write_guard(universe, l))
        }

        fn new_generic_table() -> GenericTable {
            let table = GenericTable::new(Table::new());
            let mut table = table #(.add_column({
                fn prototyper() -> GenericColumn {
                    GenericColumn {
                        name: #COL_NAME_STR,
                        stored_type_name: column_format::#COL_NAME,
                        data: {
                            type CT = #COL_TYPE2;
                            Box::new(CT::new()) as Box<AnyCol>
                        },
                        prototyper,
                        serializer_factory: serializer_factories::#COL_NAME2,
                    }
                }
                prototyper
            }))*;
            table.add_init(register_foreign_trackers);
            table
        }

        /// Register the table onto its domain.
        pub fn register() {
            new_generic_table().register();
        }

        impl<'u> Write<'u> {
            /// Borrow a `Read` lock from a `Write` lock.
            ///
            /// You might want to implement methods on your table locks. Some of these will
            /// only require an immutable reference, but you might still want to use them on
            /// `Write` locks. Using `$table.as_read()` lets those methods be reasonably
            /// accessible, without code duplication.
            pub fn as_read<'r>(&'u self) -> Read<'r>
            where 'u: 'r
            {
                Read {
                    _lock: BiRef::Right(&*self._lock),
                    _table: &self._table,
                    #(
                        #COL_NAME: RefA::new(self.#COL_NAME2.deref()),
                    )*
                }
            }
        }
    }};


    let foreign_cols = || table.cols.iter().filter(|x| x.foreign);
    let COL_TRACK_EVENTS: &Vec<_> = &foreign_cols()
        .map(|x| i(format!("track_{}_events", x.name)))
        .collect();
    let _COL_TRACK_ELEMENTS: &Vec<_> = &foreign_cols()
        .map(|x| pp::ty_to_string(&*x.element))
        .map(i)
        .collect();
    out! {
        true || table.consistent => ["tracking"] {
            #(
                /// You must implement [`Tracker`] on this struct to maintain consistency by responding to
                /// structural changes on the foreign table.
                #[allow(non_camel_case_types)] // We do not want to guess at the capitalization.
                pub struct #COL_TRACK_EVENTS;
            )*

            fn register_foreign_trackers(_universe: &Universe) {
                #({
                    _universe.register_tracker(
                        #COL_TRACK_EVENTS,
                    );
                })*
            }
        };
        /*["no tracking"] {
            fn register_foreign_trackers(_universe: &Universe) {
            }
        }*/ // FIXME: Some of my stuff is registering trackers to tables that shouldn't have them? o_O But it works!?
    };

    let TABLE_PATH_STR = format!("{}/{} version={},cols={},#={}",
        TABLE_DOMAIN_STR, TABLE_NAME_STR, table.version, table.cols.len(), table.hash_names());
    out! { ["`context!` duck-type implementation"] {
        use self::v11::context::Lockable;

        unsafe impl<'u> Lockable<'u> for Write<'u> {
            const TYPE_NAME: &'static str = concat!("mut v11/table/", #TABLE_PATH_STR);
            fn lock(universe: &'u Universe) -> Self { write(universe) }
        }
        unsafe impl<'u> Lockable<'u> for Read<'u> {
            const TYPE_NAME: &'static str = concat!("ref v11/table/", #TABLE_PATH_STR);
            fn lock(universe: &'u Universe) -> Self { read(universe) }
        }

        // Can't do Edit because it has multiple lifetimes :(
    }};

    if table.save && !table.derive.clone { panic!("#[save] requires #[row_derive(Clone)]"); }

    out! {
        // FIXME: Use Serde, and encode by columns instead.
        table.save => ["Save"] {
            use std::fmt;
            use self::v11::de_help::Hole;
            use self::v11::serde::de::{Visitor, MapAccess, Deserializer, Error};
            impl<'u> Write<'u> {
                pub fn deserialize<'de, D>(
                    &mut self,
                    deserializer: D
                ) -> Result<(), D::Error>
                where
                    D: Deserializer<'de>
                {
                    deserializer.deserialize_map(self)
                }
            }
            impl<'a, 'u: 'a, 'de> Visitor<'de> for &'a mut Write<'u> {
                type Value = ();

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a table")
                }

                #[allow(unused_mut)]
                fn visit_map<A>(mut self, mut _map: A) -> Result<Self::Value, A::Error>
                where
                    A: MapAccess<'de>
                {
                    // Due to lack of mangling, we must prefix *all* of our local vars with a `_`
                    // so as to not collide with a column name!
                    //   guicursor=n-v-c-sm:block,i-ci-ve:ver25,r-cr-o:hor20

                    // FIXME: Were we this careful in other methods?
                    let _map = &mut _map;
                    let mut _fmt = Hole::val("table:fmt", 0u32);
                    let mut _domain = Hole::val("table:domain", TABLE_DOMAIN.0 as &'de str);
                    let mut _name = Hole::val("table:name", TABLE_NAME.0 as &'de str);
                    let mut _schema_version = Hole::val("table:schema_version", 0);
                    let mut _selection = Hole::<SelectOwned<Row>>::new("table:selection");
                    #(
                        let mut #COL_NAME2 = Hole::<#COL_TYPE>::new(#COL_NAME_STR);
                    )*
                    while let Some(_key) = _map.next_key()? {
                        match _key {
                            "table:fmt" => _fmt.expect(_map)?,
                            "table:domain" => _domain.expect(_map)?,
                            "table:name" => _name.expect(_map)?,
                            "table:schema_version" => _schema_version.expect(_map)?,
                            "table:selection" => _selection.fill(_map)?,
                            #(
                                #COL_NAME_STR => #COL_NAME3.fill(_map)?,
                            )*
                            _ => return Err(A::Error::custom(format!("Unknown field {:?}", _key))), // FIXME: There's a better error type.
                        }
                    }

                    {
                        _fmt.expected::<A>()?;
                        _domain.expected::<A>()?;
                        _name.expected::<A>()?;
                    }
                    let _selection = _selection.take::<A>()?;

                    // FIXME: #[allow(dead_code)] or something?
                    let mut prev_len = None;
                    #(
                        let #COL_NAME4 = {
                            let c = #COL_NAME5.take::<A>()?;
                            let len = c.inner().len();
                            if let Some(prev_len) = prev_len {
                                if prev_len != len {
                                    return Err(A::Error::custom(format!("columns have mismatched lengths")));
                                }
                            } else {
                                prev_len = Some(len);
                            }
                            c
                        };
                    )*
                    let len = prev_len.unwrap_or(0);

                    let mut remapped = Vec::with_capacity(len);
                    let _iter = _selection
                        .as_slice()
                        .iter_or_all(RowRange {
                            start: FIRST,
                            end: GenericRowId::from_usize(len),
                        }.iter_slow());
                    #(
                        type F = #FOREIGN_ELEMENTS;
                        use std::marker::PhantomData;
                        fn get_flush<T: GetTableName>(
                            _: PhantomData<GenericRowId<T>>,
                            gt: &GenericTable,
                        ) -> &Flush<T> {
                            gt.table.get_flush_ref().downcast_ref().unwrap()
                        }
                        let gt: &RwLock<GenericTable> = F::get_generic_table(self._universe);
                        let gt = gt.read().unwrap();
                        let phantom: PhantomData<F> = PhantomData;
                        let #FOREIGN_NAME_NONCE = (
                            &*gt,
                            get_flush(phantom, &*gt),
                        );
                    )*
                    for _i in _iter {
                        let mut row = Row {
                            #(
                                #COL_NAME5: #COL_NAME6[_i],
                            )*
                        };
                        #(
                            row.#FOREIGN_LOCAL_COL = #FOREIGN_NAME_NONCE.1
                                .remap(row.#FOREIGN_LOCAL_COL2)
                                .unwrap_or_else(|| panic!("Row {:?} has no remapping", row.#FOREIGN_LOCAL_COL3));
                        )*
                        let _rowid = self.push(row);
                        if GUARANTEES.consistent {
                            remapped.push((_i, _rowid));
                        }
                    }
                    {
                        let mut flush: &mut Flush<Row> = self._table.get_flush_mut().downcast_mut().unwrap();
                        flush.set_remapping(&remapped);
                    }
                    Ok(())
                }
            }
            mod serializer_factories {
                use super::v11::serde::ser::{Serialize, Serializer, SerializeSeq};
                use super::v11::erased_serde::Serialize as ErasedSer;
                use super::v11::tracking::{Select, SelectAny, SelectOwned};
                #(
                    pub fn #COL_NAME<'a, 'b>(col: &'a GenericColumn, sel: &'a SelectAny<'b>) -> BoxedSerialize<'a> {
                        type ColType = #COL_TYPE;

                        let col = col.data.downcast_ref::<ColType>().expect("Column mismatch");
                        let sel = match sel {
                            Select::All => Select::All,
                            Select::These(rows) => Select::These(rows.downcast().expect("RowId mismatch")),
                        };
                        struct CS<'a> {
                            col: &'a ColType,
                            sel: Select<&'a [super::RowId]>,
                        }
                        impl<'a> Serialize for CS<'a> {
                            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
                            where
                                S: Serializer,
                            {
                                // FIXME: Specialize.
                                match self.sel {
                                    Select::All => {
                                        // FIXME: This ougtha be `transmute`.
                                        let col = self.col.inner();
                                        let len = col.len();
                                        let mut seq = serializer.serialize_seq(Some(len))?;

                                        for i in 0..col.len() {
                                            let e = unsafe { col.unchecked_index(i) };
                                            seq.serialize_element(e)?;
                                        }
                                        seq.end()
                                    },
                                    Select::These(rows) => {
                                        let mut seq = serializer.serialize_seq(Some(rows.len()))?;
                                        let col = self.col.inner();
                                        for i in rows {
                                            // We don't have a guarantee here!
                                            let e = col.checked_index(i.to_usize());
                                            seq.serialize_element(e)?;
                                        }
                                        seq.end()
                                    },
                                }
                            }
                        }
                        Box::new(CS { col, sel }) // There are many things that need to be erased.
                        // FIXME: We return a Box. However! We could return a SmallBox instead.
                        // Always just the two pointers!
                    }
                )*
            }
        };
        ["Saveless stubs"] {
            mod serializer_factories {
                #(
                    pub fn #COL_NAME<'a, 'b>(col: &'a GenericColumn, sel: &'a SelectAny<'b>) -> BoxedSerialize<'a> {
                        super::v11::tables::no_serializer_factory(col, sel)
                    }
                )*
            }
        };
    }

    Ok(())
}
