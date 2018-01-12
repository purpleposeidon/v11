use std::io::Write;

use quote::{Ident, Tokens};
use syntex_syntax::print::pprust as pp;

use super::table::Table;

/// Convert a string into a quote `Ident`.
fn i<S: AsRef<str>>(s: S) -> Ident {
    Ident::new(s.as_ref())
}

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

    // Info
    writeln!(out, "// Table config:")?;
    for line in format!("{:#?}", table).split('\n') {
        writeln!(out, "//   {}", line)?;
    }

    let str2i = |v: &Vec<String>| -> Vec<Ident> { v.iter().map(i).collect() };

    // "name": ["element"; "col_type"],
    use ::table::Col;
    let COL_NAME_STR: &Vec<_> = &table.cols.iter().map(|x| pp::ident_to_string(x.name)).collect();
    let COL_ELEMENT_STR: &Vec<_> = &table.cols.iter().map(|x| pp::ty_to_string(&*x.element)).collect();
    let COL_TYPE_STR: &Vec<_> = &table.cols.iter().map(|x| format!("Col<{}, Row>", pp::ty_to_string(&*x.colty))).collect();
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

    // Work around for things like #(#COL_NAME: row.#COL_NAME)*
    let COL_NAME2 = COL_NAME;
    let COL_NAME3 = COL_NAME;
    let COL_NAME4 = COL_NAME;
    let COL_TYPE2 = COL_TYPE;

    let TABLE_NAME_STR = table.name.clone();
    let TABLE_VERSION = table.version;
    let TABLE_DOMAIN = i(table.domain.clone());
    out! { ["Imports & header data"] {
        #[allow(unused_imports)]
        use super::*;

        use v11;
        use self::v11::{Universe, DomainName};
        use self::v11::intern::{self, PBox};
        use self::v11::tables::*;
        use self::v11::columns::*;
        use self::v11::index::CheckedIter;

        // Having them automatically imported is a reasonable convenience.
        #[allow(unused_imports)]
        use self::v11::storage::*;

        pub const TABLE_NAME: TableName = TableName(#TABLE_NAME_STR);
        pub const TABLE_DOMAIN: DomainName = super::#TABLE_DOMAIN;
        pub const VERSION: usize = #TABLE_VERSION;

        #[allow(non_upper_case_globals)]
        mod column_format {
            #(pub const #COL_NAME: &'static str = #COL_FORMAT;)*
        }
    }}

    let DERIVE_CLONE = quote_if(table.clone, quote! {
        #[derive(Clone)]
    });
    let DERIVE_ENCODING = quote_if(table.save, quote! { #[derive(RustcEncodable, RustcDecodable)] });
    let DERIVE_ENCODING_W = quote_if(table.save, quote! { #[derive(RustcEncodable)] });
    let DERIVE_DEBUG = quote_if(table.debug, quote! { #[derive(Debug)] });

    let ROW_ID_TYPE = i(&table.row_id);
    out! { ["Indexing"] {
        /// This is the type used to index into `#TABLE_NAME`'s columns.
        /// It is typed specifically for this table.
        pub type RowId = GenericRowId<Row>;
        /// The internal index type, which also limits the maximum number of rows.
        pub type RawType = #ROW_ID_TYPE;

        /// An undefined index value to be used for default values.
        pub const INVALID: RowId = RowId {
            i: ::std::usize::MAX as RawType,
            t: ::std::marker::PhantomData,
        };

        /// A reference to the first row. Is invalid if there is no rows.
        pub const FIRST: RowId = RowId {
            i: 0,
            t: ::std::marker::PhantomData,
        };

        /// Creates an index into the `i`th row.
        pub fn at(i: #ROW_ID_TYPE) -> RowId { RowId::new(i) }
    }}

    out! { ["The `Row` struct"] {
        /**
         * A structure holding a copy of each column's data. This is used to pass entire rows around through methods;
         * the actual table is column-based, so eg `read.column[index]` is the standard method of accessing rows.
         * */
        #DERIVE_CLONE
        #DERIVE_ENCODING
        #DERIVE_DEBUG
        // Presumably too fat for Copy.
        // FIXME: How about RowDerive()?
        pub struct Row {
            #(#COL_ATTR pub #COL_NAME: #COL_ELEMENT,)*
        }
        impl GetTableName for Row {
            type Idx = RawType;
            fn get_domain() -> DomainName { TABLE_DOMAIN }
            fn get_name() -> TableName { TABLE_NAME }
        }

        /// A row holding a reference to each 
        #DERIVE_DEBUG
        #DERIVE_ENCODING_W
        // FIXME: How about RowDerive()?
        // FIXME: Maybe this should be asked for instead?
        pub struct RowRef<'a> {
            #(#COL_ATTR pub #COL_NAME: &'a #COL_ELEMENT,)*
        }

        // `struct RowMut` would require keeping the primary key a ref.
    }};

    let COL_MUT: &Vec<_> = &table.cols.iter()
        .map(|x| if x.indexed { "EditA" } else { "MutA" })
        .map(i)
        .collect();

    out! { ["Table locks"] {
        /**
         * The table, locked for reading.
         * */
        pub struct Read<'u> {
            _lock: ::std::sync::RwLockReadGuard<'u, GenericTable>,
            #(pub #COL_NAME: RefA<'u, #COL_TYPE>,)*
        }
        /**
         * The table, locked for writing.
         * */
        pub struct Write<'u> {
            _lock: ::std::sync::RwLockWriteGuard<'u, GenericTable>,
            #(pub #COL_NAME: #COL_MUT<'u, #COL_TYPE>,)*
        }

        impl<'u> LockedTable for Read<'u> {
            type Row = Row;
            fn len(&self) -> usize { self.len() }
        }
        impl<'u> LockedTable for Write<'u> {
            type Row = Row;
            fn len(&self) -> usize { self.len() }
        }
    }};


    let GET_ROW = quote_if(table.clone, quote! {
        use self::v11::index::Checkable;
        /** Retrieves a structure containing a clone of the value in each column. (R/W) */
        pub fn get_row<R: Checkable<Row=Row>>(&self, index: R) -> Row {
            // FIXME: get_row + FreeList == ??
            let index = index.check(self);
            Row {
                #(#COL_NAME: self.#COL_NAME2[index].clone(),)*
            }
        }
    });

    let DUMP_ROWS = quote_if(table.clone && !table.consistent, quote! {
        /** Allocates a Vec filled with every Row in the table. (R/W) */
        pub fn dump(&self) -> Vec<Row> {
            let mut ret = Vec::with_capacity(self.len());
            for i in self.iter() {
                ret.push(self.get_row(i));
            }
            ret
        }
    });

    let RW_FUNCTIONS_CONSISTENT = quote! {
        /** Returns an iterator over each nonrow in the table that is not marked for deletion.
         * (R/W) */
        pub fn iter(&self) -> ConsistentIter<Self> {
            let checked_iter = CheckedIter::from(self, self.row_range());
            ConsistentIter::new(checked_iter, &self._lock.free)
        }

        /** Returns true if `i` is a valid RowId. */
        pub fn contains(&self, index: RowId) -> bool {
            index.to_usize() < self.len() && !self._lock.free.contains_key(&index.to_usize())
        }
    };
    let RW_FUNCTIONS_INCONSISTENT = quote! {
        /** Returns an iterator over each row in the table. (R/W) */
        pub fn iter(&self) -> CheckedIter<Self> {
            CheckedIter::from(self, self.row_range())
        }

        /** Returns true if `i` is a valid RowId. */
        pub fn contains(&self, index: RowId) -> bool {
            index.to_usize() < self.len()
        }

        /** Retrieves a structure containing a reference to each value in each column. (R/W) */
        pub fn get_row_ref(&self, index: RowId) -> RowRef {
            RowRef {
                #(#COL_NAME: &self.#COL_NAME2[index],)*
            }
        }

        #GET_ROW

        #DUMP_ROWS

        // FIXME: Join
    };
    let RW_FUNCTIONS_BOTH = quote! {
        /** Returns the number of rows in the table. (R/W) */
        // And assumes that the columns are all the same length.
        // But there shouldn't be any way to break that invariant.
        pub fn len(&self) -> usize {
            self.#COL0.deref().data().len()
        }

        /// Gets the last `RowId`.
        pub fn last(&self) -> Option<RowId> {
            // FIXME: FreeList!
            let r = self.len();
            if r == 0 {
                None
            } else {
                Some(RowId::from_usize(r - 1))
            }
        }

        pub fn row_range(&self) -> RowRange<RowId> {
            (RowId::new(0)..RowId::from_usize(self.len())).into()
        }
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
        }
    }};

    out! {
        table.consistent => ["Change tracking"] {
            use v11::tracking::Tracker;

            /// Add a tracker.
            pub fn register_tracker(universe: &Universe, tracker: Box<Tracker + Send + Sync>) {
                let mut gt = Row::get_generic_table(universe).write().unwrap();
                gt.add_tracker(tracker);
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

                pub fn delete(&mut self, row: RowId) {
                    self._lock.delete.push(row.to_usize());
                    self.event_delete(row.to_usize());
                    // FIXME: Updating indexes? Maybe we can self-track?
                }
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
    }

    out! {
        table.consistent => ["Extra drops"] {
            /// Prevent moving out to improve `RefA` safety.
            impl<'u> Drop for Read<'u> {
                fn drop(&mut self) {}
            }
        };
        ["Extra drops"] {
            /// Prevent moving out to improve `RefA` safety.
            impl<'u> Drop for Read<'u> {
                fn drop(&mut self) {}
            }

            /// Prevent moving out to improve `MutA` safety.
            impl<'u> Drop for Write<'u> {
                fn drop(&mut self) {}
            }
        };
    }

    out! {
        table.consistent => ["event logging for consistent tables"] {
            impl<'u> Write<'u> {
                fn event_cleared(&mut self) { self._lock.dirty().cleared = true; }
                fn event_add(&mut self, i: usize) { self._lock.dirty().add.push(i); }
                fn event_delete(&mut self, i: usize) { self._lock.dirty().delete.push(i); }
                fn event_add_reserve(&mut self, n: usize) { self._lock.dirty().add.reserve(n) }
                fn event_delete_reserve(&mut self, n: usize) { self._lock.dirty().delete.reserve(n) }
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
            pub fn reserve(&mut self, additional: usize) {
                #(self.#COL_NAME.deref_mut().data_mut().reserve(additional);)*
                self.event_add_reserve(additional);
            }

            /// Removes every row from the table.
            pub fn clear(&mut self) {
                #(self.#COL_NAME.deref_mut().data_mut().clear();)*
                self.event_cleared();
            }

            /// Not really 'safe', but it's private.
            /// Add a Row to the end of the table, without checking the free-list.
            #[inline]
            fn push_end_unchecked(&mut self, row: Row) -> RowId {
                #(self.#COL_NAME.deref_mut().data_mut().push(row.#COL_NAME2);)*
                let rowid = self.last().unwrap();
                self.event_add(rowid.to_usize());
                rowid
            }

        }
    }};

    out! {
        table.sorted || !table.immutable => ["swap_row() for retain & merge"] {
            impl<'u> Write<'u> {
                #[inline]
                // Making this public would break many guarantees!
                fn swap_row(&mut self, i: RowId, row: &mut Row) {
                    use std::mem::swap;
                    #(swap(&mut self.#COL_NAME.deref_mut()[i], &mut row.#COL_NAME2);)*
                }
            }
        };
    };

    out! {
        table.sorted => ["row pushing for sorted tables"] {
            impl<'u> Write<'u> {
                pub fn retain<F: Fn(&Self, I) -> bool>(&mut self, f: F) {
                    unimplemented!();
                    let _check_my_source = Vec::retain;
                }

                pub fn merge<IT: Iterator<Item=Row>, I: Into<AssertSorted<IT>>>(&mut self, rows: I)
                where IT: IntoIterator<Row>
                {
                    // This is actually a three-way merge. Joy!
                    // We have three things: the table, the rug, and the iter.
                    // We get 'next' by merging the rug and the iter.
                    // Whenever something gets bumped off the table, it is pushed onto the rug.
                    let mut rug = ::std::collections::VecDeque::new();
                    let mut iter = rows.into().peekable();
                    let mut i = FIRST;
                    self.reserve(iter.size_hint().0);
                    loop {
                        enum Side { Rug, Iter }
                        let side = {
                            let (next, side) = match (rug.last(), iter.peek()) {
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
                            if i < self.len() && &self.get_row(i) <= next {
                                None
                            } else {
                                Some(side)
                            }
                        };
                        // { Some(Rug), Some(Iter), None } × { more table, table finished }
                        if let Some(side) = side {
                            let next = match side {
                                Side::Rug => rug.pop_front(),
                                Side::Iter => iter.next(),
                            }.unwrap();
                            if i < self.len() {
                                // swap the row
                                self.swap_row(i, next);
                                if cfg!(debug) {
                                    // We know that `rug` is sorted.
                                    // But what if `next < rug.front()`? We'd break the ordering!
                                    // Well, we never arrive at that situation.
                                    // ALWAYS: primary[i] < rug.front() && primary[i] < iter.peek()
                                    if let Some(rug_front) = rug.front() {
                                        assert!(next >= rug_front);
                                        assert!(next >= rug.back().unwrap());
                                    }
                                }
                                rug.push_back(next);
                            } else {
                                self.push_end(row);
                            }
                        } // else: no change
                        i = i.next();
                    }
                }
            }
        };
        ["row pushing for unsorted tables"] {
            impl<'u> Write<'u> {
                #[inline]
                fn set_row_raw(&mut self, index: RowId, row: Row) {
                    #(self.#COL_NAME.deref_mut()[index] = row.#COL_NAME2;)*
                }

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
                pub fn push(&mut self, row: Row) -> RowId {
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
                        self.set_row_raw(old, row);
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
                _lock,
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
            let table = GenericTable::new(TABLE_DOMAIN, TABLE_NAME);
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
    }};


    let COL_TRACK_EVENTS: &Vec<_> = &table.cols.iter()
        .filter(|x| x.foreign)
        .map(|x| i(format!("track_{}_events", x.name)))
        .collect();
    let COL_TRACK_ELEMENTS: &Vec<_> = &table.cols.iter()
        .filter(|x| x.foreign)
        .map(|x| pp::ty_to_string(&*x.element))
        .map(i)
        .collect();
    out! { ["tracking"] {
        #(
            /// `Tracker` must be implemented on this struct to maintain consistency by responding to
            /// structural tables on the foreign table.
            #[allow(non_camel_case_types)]
            pub struct #COL_TRACK_EVENTS;
        )*
        fn register_foreign_trackers(_universe: &Universe) {
            #(
                let bx = Box::new(#COL_TRACK_EVENTS) as Box<Tracker + Sync + Send>;
                #COL_TRACK_ELEMENTS::register_tracker(_universe, bx);
            )*
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
            #[doc(hdiden)] #[inline] pub fn lock_name() -> &'static str { concat!("ref ", #TABLE_NAME_STR) }
        }
    }};

    out! {
        // FIXME: Track free-list
        table.save => ["Save"] {
            use rustc_serialize::{Decoder, Decodable, Encoder, Encodable};

            impl<'u> Read<'u> {
                /// Row-based encoding.
                pub fn encode_rows<E: Encoder>(&self, e: &mut E) -> Result<(), E::Error> {
                    use rustc_serialize::Encoder;
                    e.emit_seq(self.len(), |e| {
                        for i in self.iter() {
                            let row = self.get_row_ref(i);
                            e.emit_seq_elt(i.to_usize(), |e| {
                                row.encode(e)
                            })?;
                        }
                        Ok(())
                    })
                }

                /* -- This is kind of not possible to do due to funky bits in Col & BoolVec
                 * that shouldn't be serialized. Serde'd make it possible?
                /// Column-based encoding.
                pub fn encode_columns<E: Encoder>(&self, e: &mut E) -> Result<(), E::Error> {
                    use rustc_serialize::Encoder;
                    e.emit_struct(TABLE_NAME, 1 + #COL_COUNT, |e| {
                        let expect_rows = self.len();
                        e.emit_struct_field("_expect_rows", 0, |e| expect_rows.encode(e))?;
                        let mut col = 1;
                        #({
                            e.emit_struct_field(#COL_NAME, col, |e| {

                            })?;
                            col += 1;
                        })*
                    });
                    #[derive(RustcEncodable)]
                    struct Saver<'a> {
                        _expect_rows: usize,
                        #(#COL_NAME: &'a #COL_TYPE_RAW,)*
                    }
                    let saver = Saver {
                        _expect_rows: self.len(),
                        #(#COL_NAME: &self.#COL_NAME2.data,)*
                    };
                    saver.encode(e)
                }
                */
            }

            impl<'u> Write<'u> {
                /// Row-based decoding. Clears the table before reading, and clears the table if
                /// there is an error.
                pub fn decode_rows<D: Decoder>(&mut self, d: &mut D) -> Result<(), D::Error> {
                    use rustc_serialize::Decoder;
                    self.clear();
                    let caught = d.read_seq(|e, count| {
                        self.reserve(count);
                        for i in 0..count {
                            let row = e.read_seq_elt(i, Row::decode)?;
                            self.push(row);
                        }
                        Ok(())
                    });
                    if caught.is_err() {
                        self.clear();
                    }
                    caught
                }
                /*
                /// Column-based decoding. Clears the table before reading, and clears the table if
                /// there is an error.
                pub fn decode_columns<D: Decoder>(&mut self, d: &mut D) -> Result<(), D::Error> {
                    self.clear();
                    #[derive(RustcDecodable)]
                    struct Saver {
                        _expect_rows: usize,
                        #(#COL_NAME: #COL_TYPE_RAW,)*
                    }
                    let saver = Saver::decode(d)?;

                    if self.len() != saver._expect_rows {
                        println!("have {}, expect {}", self.len(), saver._expect_rows);
                        Err(d.error("mismatched row count"))
                    } else if self.inconsistent_columns() {
                        Err(d.error("inconsistent column heights"))
                    } else {
                        #(self.#COL_NAME.data = saver.#COL_NAME2;)*
                        Ok(())
                    }
                }

                fn inconsistent_columns(&self) -> bool {
                    let len = self.len();
                    #({
                        if len != self.#COL_NAME.len() {
                            return true;
                        }
                    })*
                    false
                }*/
            }
        };
    }

    Ok(())
}
