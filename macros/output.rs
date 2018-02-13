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
    let COL_TYPE2 = COL_TYPE;

    let TABLE_NAME_STR = table.name.clone();
    #[allow(unused)]
    let TABLE_VERSION = table.version;
    let TABLE_DOMAIN = i(table.domain.clone());
    let GUARANTEES = {
        let CONSISTENT = table.consistent;
        quote! {
            Guarantee {
                consistent: #CONSISTENT,
            }
        }
    };
    //: bool = table.sort_key.is_some();
    out! { ["Imports & header data"] {
        #[allow(unused_imports)]
        use super::*;

        use v11;
        use self::v11::Universe;
        use self::v11::domain::DomainName;
        use self::v11::intern::{self, PBox, BiRef};
        use self::v11::tables::*;
        use self::v11::columns::*;
        use self::v11::index::{CheckedIter, Checkable};

        #[allow(unused_imports)] use self::v11::storage::*; // A reasonable convenience for the user.
        #[allow(unused_imports)] use self::v11::joincore::*;
        #[allow(unused_imports)] use self::v11::map_index::BTreeIndex;
        #[allow(unused_imports)] use self::v11::Action;
        #[allow(unused_imports)] use self::v11::tracking::Tracker;
        #[allow(unused_imports)] use std::collections::VecDeque;
        #[allow(unused_imports)] use std::cmp::Ordering;

        pub const TABLE_NAME: TableName = TableName(#TABLE_NAME_STR);
        pub const TABLE_DOMAIN: DomainName = super::#TABLE_DOMAIN;
        pub const VERSION: u64 = #TABLE_VERSION;
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
            t: ::std::marker::PhantomData,
        };

        /// An index value to be used for default values.
        /// Note that it may become valid if the table is full!
        pub const INVALID: RowId = RowId {
            i: ::std::usize::MAX as RawType,
            t: ::std::marker::PhantomData,
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
            "Clone" | "RustcEncodable" | "RustcDecodable" => false,
            _ => true,
        }).collect();

    out! { ["The `Row` struct"] {
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
            self._lock.free.get(&idx.to_usize()).is_some()
        }
    });
    out! { ["Table locks"] {
        /**
         * The table, locked for reading.
         * */
        pub struct Read<'u> {
            _lock: BiRef<::std::sync::RwLockReadGuard<'u, GenericTable>, &'u GenericTable, GenericTable>,
            #(pub #COL_NAME: RefA<'u, #COL_TYPE>,)*
        }

        /**
         * The table, locked for writing.
         * */
        pub struct Write<'u> {
            _lock: ::std::sync::RwLockWriteGuard<'u, GenericTable>,
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
                    use std::mem;
                    let me1: &Self = mem::transmute(self as *const Self);
                    let me2: &mut Self = mem::transmute(self as *mut Self);
                    let me3 = self;
                    (Edit {
                        _inner: me1,
                        #(#COL_NAME: &mut me2.#COL_NAME2,)*
                    }, EditIter::new(me3.row_range(), me3._lock.free.keys()))
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
            ConsistentIter::new(checked_iter, &self._lock.free)
        }

        /** Returns true if `i` is a valid RowId. */
        pub fn contains(&self, index: RowId) -> bool {
            index.to_usize() < self.len() && !self._lock.free.contains_key(&index.to_usize())
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
        /// Be careful calling this on consistent tables; it may include deleted rows. You can use 
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

    let ifcs = || table.cols.iter().filter(|x| x.indexed && x.foreign);

    let IFC: Vec<_> = ifcs()
        .map(|x| pp::ident_to_string(x.name))
        .map(i)
        .collect();
    let TRACK_IFC_REMOVAL: Vec<_> = ifcs()
        .map(|x| format!("track_{}_removal", x.name))
        .map(i)
        .collect();
    let IFC_ELEMENT: Vec<_> = ifcs()
        .map(|x| pp::ty_to_string(&*x.element))
        .map(i)
        .collect();

    out! {
        table.consistent => ["Change tracking"] {
            /// Add a tracker.
            pub fn register_tracker(universe: &Universe, tracker: Box<Tracker + Send + Sync>, sort_events: bool) {
                let mut gt = Row::get_generic_table(universe).write().unwrap();
                gt.add_tracker(tracker, sort_events);
            }

            impl<'a> Write<'a> {
                /// Allow the `Write` lock to be closed without flushing changes. Be careful!
                /// The changes need to be flushed eventually!
                pub fn no_flush(mut self) {
                    self._lock.need_flush = false;
                }

                /// Propagate all changes
                pub fn flush(mut self, universe: &Universe) {
                    if self._lock.skip_flush() { return; }
                    let mut flush = self._lock.acquire_flush();
                    ::std::mem::drop(self);
                    flush.flush(universe);
                    let mut gt = Row::get_generic_table(universe).write().unwrap();
                    flush.restore(&mut gt);
                }

                pub fn delete<I: CheckId>(&mut self, row: I) {
                    unsafe {
                        let i = row.check(self).to_usize();
                        self.delete_raw(i);
                        self.event_delete(i);
                    }
                }

                unsafe fn delete_raw(&mut self, i: usize) {
                    #(
                        self.#COL_NAME.deref_mut().inner_mut().deleted(i);
                    )*
                }

                #(
                    /// `deleted` is a list of removed foreign keys.
                    pub fn #TRACK_IFC_REMOVAL(&mut self, deleted: &[usize]) {
                        for deleted_foreign in deleted {
                            type E = #IFC_ELEMENT;
                            let deleted_foreign = E::from_usize(*deleted_foreign);
                            loop {
                                // It'd be nicer to keep the iterator around, but we immediately
                                // invalidate it. We could collect it into a Vec?
                                let kill = if let Some(kill) = self.#IFC.deref().inner().find(deleted_foreign).next() {
                                    // FIXME: Add a 'Sorted' wrapping TCol that exposes find() using binary search.
                                    kill
                                } else {
                                    break;
                                };
                                self.delete(kill);
                                if cfg!(test) && self.contains(kill) {
                                    panic!("Deletion failed");
                                }
                            }
                        }
                    }
                )*
            }

            /// Makes sure the flush requirement has been acknowledged
            impl<'a> Drop for Write<'a> {
                fn drop(&mut self) {
                    if self._lock.need_flush {
                        panic!("Changes to {} were not flushed", TABLE_NAME);
                    }
                }
            }
        };
        ["fake flush"] {
            impl<'u> Write<'u> {
                /// This table does not need to be flushed; this method is here as a
                /// convenience for macros.
                pub fn flush(self, _universe: &Universe) {}

                // "shouldn't" get called; could happen if the table kind changes between
                // serializations. This is a stub.
                unsafe fn delete_raw(&mut self, _i: usize) {
                    panic!("Unexpected call to delete_raw");
                }
            }
        };
    }

    let sorted_foreign = || table.cols.iter().filter(|x| Some(x.name) == table.sort_key && x.foreign);
    let TRACKED_SORTED_COL: &Vec<_> = &sorted_foreign()
        .map(|x| i(pp::ident_to_string(x.name)))
        .collect();
    let TRACK_SORTED_COL_EVENTS: &Vec<_> = &sorted_foreign()
        .map(|x| i(format!("track_sorted_{}_removal", x.name)))
        .collect();
    out! { ["track sorted events"] {
        impl<'u> Write<'u> {
            #(
                /// This is a table sorted by a foreign key. This function removes all the keys
                /// listed in `remove`, which must also be sorted.
                pub fn #TRACK_SORTED_COL_EVENTS(&mut self, remove: &[usize]) {
                    if remove.is_empty() || self.len() == 0 { return; }
                    let mut core = JoinCore::new(remove.iter().map(|x| *x));
                    self.merge0(move |me, rowid| {
                        use std::iter::empty;
                        let foreign = me.#TRACKED_SORTED_COL[rowid].to_usize();
                        match core.cmp(&foreign) {
                            Join::Match(_) => Action::Continue { remove: true, add: empty() },
                            Join::Next => Action::Continue { remove: false, add: empty() },
                            Join::Stop => Action::Break,
                        }
                    });
                }
            )*
        }
    }};

    for col in &table.cols {
        if !col.foreign_auto { continue; }
        let TRACK_EVENTS = i(format!("track_{}_events", col.name));
        let DELEGATE = i(if Some(col.name) == table.sort_key {
            format!("track_sorted_{}_removal", col.name)
        } else if col.indexed {
            format!("track_{}_removal", col.name)
        } else {
            panic!("`#[foreign_auto]` can only be used on columns with `#[index]` or `#[sort_key]`.");
        });
        out! { ["foreign_auto"] {
            impl Tracker for #TRACK_EVENTS {
                fn cleared(&mut self, universe: &Universe) {
                    let mut lock = write(universe);
                    lock.clear();
                    lock.flush(universe);
                }

                fn track(&mut self, universe: &Universe, deleted_rows: &[usize], _added_rows: &[usize]) {
                    if deleted_rows.is_empty() { return; }
                    let mut lock = write(universe);
                    lock.#DELEGATE(deleted_rows);
                    lock.flush(universe);
                }
            }
        }};
    }
    for col in &table.cols {
        let COLUMN = i(pp::ident_to_string(col.name));
        let FIND = i(format!("find_{}", col.name));
        let ELEMENT = i(pp::ty_to_string(&col.element));
        out! {
            col.indexed => [""] {
                impl<'u> Read<'u> {
                    pub fn #FIND(&self, e: #ELEMENT) -> Option<RowId> {
                        self.#COLUMN.deref().inner().find(e).next()
                    }
                }
                impl<'u> Write<'u> {
                    pub fn #FIND(&self, e: #ELEMENT) -> Option<RowId> {
                        self.as_read().#FIND(e)
                    }
                }
            };
            // FIXME: Binary search
            // FIXME: It'd be better to do `mytable.column.find(blah)`.
            /*if col.sort_key => [""] {
                impl<'u> Read<'u> {
                    fn #FIND(&self, e: &#ELEMENT) -> Option<RowId> {
                    }
                }
            };*/
            [""] {};
        }
    }

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
                fn event_cleared(&mut self) { self._lock.cleared = true; }
                fn event_add(&mut self, i: usize) { self._lock.add(i); }
                fn event_delete(&mut self, i: usize) { self._lock.delete(i); }
                fn event_add_reserve(&mut self, n: usize) { self._lock.add_reserve(n) }
                fn event_delete_reserve(&mut self, n: usize) { self._lock.delete_reserve(n) }
            }
        };
        ["event ignoring for inconsistent_columns tables"] {
            impl<'u> Write<'u> {
                #[inline] fn event_cleared(&mut self) {}
                #[inline] fn event_add(&mut self, _: usize) {}
                #[inline] fn event_delete(&mut self, _: usize) {}
                #[inline] fn event_add_reserve(&mut self, _: usize) {}
                #[inline] fn event_delete_reserve(&mut self, _: usize) {}
            }
        };
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
                self.event_add(rowid.to_usize());
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

                /// `log` will be called with the new RowId of each new row.
                pub fn merge_logged<IT, I, L>(&mut self, rows: I, mut log: L)
                where
                    IT: Iterator<Item=Row>,
                    I: Into<AssertSorted<IT>>,
                    L: FnMut(&Self, RowId),
                {
                    // This is actually a three-way merge. Joy!
                    // We have three things: the table, the rug, and the iter.
                    // We get 'next' by merging the rug and the iter.
                    // Whenever something gets bumped off the table, it is pushed onto the rug.
                    // The rug is sorted because the table is sorted.
                    #[derive(Copy, Clone)]
                    enum Side { Rug, Iter }
                    // The log needs to know if a Row is new or not.
                    let mut rug: VecDeque<(Side, Row)> = VecDeque::new();
                    let mut iter = rows.into().into_iter().peekable();
                    let mut i = FIRST;
                    self.reserve(iter.size_hint().0);
                    loop {
                        let side = {
                            let rug_back = rug.back()
                                .map(|&(ref _side, ref row)| row.as_ref());
                            let iter_peek = iter.peek()
                                .map(Row::as_ref);
                            let (next, side) = match (rug_back, iter_peek) {
                                (None, None) => break,
                                (None, Some(il)) => (il, Side::Iter),
                                (Some(rl), None) => (rl, Side::Rug),
                                (Some(rl), Some(il)) => {
                                    if rl <= il {
                                        (rl, Side::Rug) // '<=', not '<', for sort stability
                                    } else {
                                        (il, Side::Iter)
                                    }
                                },
                            };
                            // So, "why do use rug.back()?" Because the rug is already the table's
                            // contents, just temporarily displaced.
                            if i.to_usize() < self.len() && self.get_row_ref(i) <= next {
                                None
                            } else {
                                Some(side)
                            }
                        };
                        // { Some(Rug), Some(Iter), None } × { more table, table finished }
                        if let Some(side) = side {
                            let (src, mut next) = match side {
                                Side::Rug => rug.pop_front().unwrap(),
                                Side::Iter => (Side::Iter, iter.next().unwrap()),
                            };
                            if i.to_usize() < self.len() {
                                // swap the row
                                self.swap_out_row(i, &mut next);
                                if cfg!(debug) {
                                    // We know that `rug` is sorted.
                                    // But what if `next < rug.front()`? We'd break the ordering!
                                    // Well, we never arrive at that situation.
                                    // ALWAYS: primary[i] < rug.front() && primary[i] < iter.peek()
                                    if let Some(rug_front) = rug.front() {
                                        assert!(next.as_ref() >= rug_front.1.as_ref());
                                        assert!(next.as_ref() >= rug.back().unwrap().1.as_ref());
                                    }
                                }
                                rug.push_back((src, next));
                            } else {
                                self.push_end_unchecked(next);
                            }
                            if let Side::Iter = src {
                                log(self, i);
                            }
                        } // else: no change
                        i = i.next();
                    }
                }
            }
        };
        !table.sorted => ["row pushing for unsorted tables"] {
            impl<'u> Write<'u> {
                /** Populate the table with data from the provided iterator. */
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
                    let next = self._lock.free.keys().next().cloned();
                    let i = if let Some(old) = next {
                        self._lock.free.remove(&old);
                        self.event_add(old);
                        let old = RowId::from_usize(old);
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
                    // unchecked index when using this.
                    i
                }

                /// Returns the RowId of the next row that would be inserted.
                pub fn next_pushed(&self) -> RowId {
                    let i = self._lock.free
                        .keys()
                        .next()
                        .cloned()
                        .unwrap_or(self.len());
                    RowId::from_usize(i)
                }

                /// Push an 'array' of values. Contiguity guaranteed!
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
        use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, LockResult, TryLockResult};

        impl Row {
            pub fn get_generic_table(universe: &Universe) -> &RwLock<GenericTable> {
                let domain_id = TABLE_DOMAIN.get_id();
                universe.get_generic_table(domain_id, TABLE_NAME)
            }
        }

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
            Read {
                _lock: BiRef::Left(_lock),
                #( #COL_NAME3: #COL_NAME4, )*
            }
        }

        /// Locks the table for reading.
        // We're too cool to be callling unwrap() all over the place.
        pub fn read(universe: &Universe) -> Read {
            read_result(universe).unwrap()
        }

        /// This is equivalent to `RwLock::read`.
        pub fn read_result<'u>(universe: &'u Universe) -> LockResult<Read<'u>> {
            let table = Row::get_generic_table(universe).read();
            intern::wrangle_lock::map_result(table, convert_read_guard)
        }

        pub fn try_read<'u>(universe: &'u Universe) -> TryLockResult<Read<'u>> {
            let table = Row::get_generic_table(universe).try_read();
            intern::wrangle_lock::map_try_result(table, convert_read_guard)
        }



        fn convert_write_guard(mut _lock: RwLockWriteGuard<GenericTable>) -> Write {
            #(let #COL_NAME = {
                let got = _lock.get_column_mut::<#COL_TYPE2>(#COL_NAME_STR, column_format::#COL_NAME2);
                unsafe {
                    #COL_MUT::new(transmute(got))
                    // See comment about transmute in `convert_read_guard()`.
                }
            };)*
            Write {
                _lock,
                #( #COL_NAME3: #COL_NAME4, )*
            }
        }

        /// Locks the table for writing.
        pub fn write<'u>(universe: &'u Universe) -> Write<'u> {
            write_result(universe).unwrap()
        }

        pub fn write_result<'u>(universe: &'u Universe) -> LockResult<Write<'u>> {
            let table = Row::get_generic_table(universe).write();
            intern::wrangle_lock::map_result(table, convert_write_guard)
        }

        pub fn try_write<'u>(universe: &'u Universe) -> TryLockResult<Write<'u>> {
            let table = Row::get_generic_table(universe).try_write();
            intern::wrangle_lock::map_try_result(table, convert_write_guard)
        }

        /// Register the table onto its domain.
        pub fn register() {
            let table = GenericTable::new(TABLE_DOMAIN, TABLE_NAME, GUARANTEES.clone());
            let mut table = table #(.add_column(
                #COL_NAME_STR,
                column_format::#COL_NAME,
                {
                    fn maker() -> PBox {
                        type CT = #COL_TYPE2;
                        Box::new(CT::new()) as PBox
                    }
                    maker
                },
            ))*;
            table.add_init(register_foreign_trackers);
            table.register();
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
                    #(
                        #COL_NAME: RefA::new(self.#COL_NAME2.deref()),
                    )*
                }
            }
        }
    }};


    let foreign_cols = || table.cols.iter().filter(|x| x.foreign);
    let COL_TRACK_EVENTS: &Vec<_> = &foreign_cols()
        .map(|x| i(format!("track_{}_events", x.name))) // FIXME: Rename to `track_{}_removal`.
        .collect();
    let COL_TRACK_ELEMENTS: &Vec<_> = &foreign_cols()
        .map(|x| pp::ty_to_string(&*x.element))
        .map(i)
        .collect();
    let SORT_EVENTS: &Vec<bool> = &foreign_cols()
        .map(|x| Some(x.name) == table.sort_key)
        .collect();
    out! { ["tracking"] {
        #(
            /// `Tracker` must be implemented on this struct to maintain consistency by responding to
            /// structural tables on the foreign table.
            #[allow(non_camel_case_types)] // We do not want to guess at the capitalization.
            pub struct #COL_TRACK_EVENTS;
        )*
        fn register_foreign_trackers(_universe: &Universe) {
            #({
                let bx = Box::new(#COL_TRACK_EVENTS) as Box<Tracker + Sync + Send>;
                type E = #COL_TRACK_ELEMENTS;
                E::register_tracker(_universe, bx, #SORT_EVENTS);
            })*
        }
    }};

    out! { ["`context!` duck-type implementation"] {
        // Hidden because `$table::read()` is shorter than `$table::Read::lock()`.
        impl<'u> Write<'u> {
            #[doc(hidden)] #[inline] pub fn lock(universe: &'u Universe) -> Self { write(universe) }
            #[doc(hidden)] #[inline] pub fn lock_name() -> &'static str { concat!("mut ", #TABLE_NAME_STR) }
        }

        impl<'u> Read<'u> {
            #[doc(hidden)] #[inline] pub fn lock(universe: &'u Universe) -> Self { read(universe) }
            #[doc(hidden)] #[inline] pub fn lock_name() -> &'static str { concat!("ref ", #TABLE_NAME_STR) }
        }
    }};

    if table.save && !table.derive.clone { panic!("#[save] requires #[row_derive(Clone)]"); }

    out! {
        // FIXME: Use Serde, and encode by columns instead.
        table.save => ["Save"] {
            use rustc_serialize::{Decoder, Decodable, Encoder, Encodable};

            impl<'u> Read<'u> {
                /// Row-based encoding.
                pub fn encode_rows<E: Encoder>(&self, e: &mut E) -> Result<(), E::Error> {
                    if !self._lock.skip_flush() {
                        panic!("Encoding unflushed table!\n{}", self._lock.unflushed_summary());
                    }
                    e.emit_struct(#TABLE_NAME_STR, 2, |e| {
                        e.emit_struct_field("free_list", 0, |e| self._lock.free.encode(e))?;
                        e.emit_struct_field("rows", 1, |e| e.emit_seq(self.len(), |e| {
                            for i in self.row_range().iter_slow() {
                                let i = unsafe { CheckedRowId::fab(i.to_raw(), self) };
                                let row = self.get_row_raw(i);
                                // FIXME: This requires clone. Column-based easily would not.
                                e.emit_seq_elt(i.to_usize(), |e| row.encode(e))?;
                            }
                            Ok(())
                        }))
                    })
                }
            }

            impl<'u> Write<'u> {
                /// Row-based decoding. Clears the table before reading, and clears the table if
                /// there is an error.
                pub fn decode_rows<D: Decoder>(&mut self, d: &mut D) -> Result<(), D::Error> {
                    if !self._lock.skip_flush() { panic!("Decoding unflushed table!"); }
                    self.clear();
                    let caught = d.read_struct(#TABLE_NAME_STR, 2, |d| {
                        self._lock.free = d.read_struct_field("free_list", 0, ::std::collections::BTreeMap::decode)?;
                        d.read_struct_field("rows", 1, |d| d.read_seq(|e, count| {
                            self.reserve(count);
                            for i in 0..count {
                                let row = e.read_seq_elt(i, Row::decode)?;
                                self.push_only_unchecked(row);
                            }
                            Ok(())
                        }))
                    });
                    if caught.is_err() {
                        self.clear();
                    }
                    // De-index deleted things.
                    // Bit lame.
                    let to_free: Vec<_> = self._lock.free.keys().map(|i| *i).collect();
                    for free in to_free.into_iter() {
                        unsafe {
                            assert!(free < self.len());
                            self.delete_raw(free);
                        }
                    }
                    caught
                }
            }
        };
    }

    Ok(())
}
