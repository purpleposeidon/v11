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
    let COL_TYPE_STR: &Vec<_> = &table.cols.iter()
        .map(|x| {
            let ct = pp::ty_to_string(&*x.colty);
            if x.indexed {
                format!("Col<BTreeIndex<{}>, Row>", ct)
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

    // Work around for things like #(#COL_NAME: row.#COL_NAME)*
    let COL_NAME2 = COL_NAME;
    let COL_NAME3 = COL_NAME;
    let COL_NAME4 = COL_NAME;
    let COL_TYPE2 = COL_TYPE;

    let TABLE_NAME_STR = table.name.clone();
    #[allow(unused)]
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
        use self::v11::index::{CheckedIter, Checkable};

        #[allow(unused_imports)] use self::v11::storage::*; // A reasonable convenience for the user.
        #[allow(unused_imports)] use self::v11::map_index::{BTreeIndex, IndexedCol};
        #[allow(unused_imports)] use self::v11::Action;
        #[allow(unused_imports)] use std::collections::VecDeque;

        pub const TABLE_NAME: TableName = TableName(#TABLE_NAME_STR);
        pub const TABLE_DOMAIN: DomainName = super::#TABLE_DOMAIN;
        pub const VERSION: usize = #TABLE_VERSION;

        #[allow(non_upper_case_globals)]
        mod column_format {
            #(pub const #COL_NAME: &'static str = #COL_FORMAT;)*
        }
    }}

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

        // FIXME: Implement `struct RowMut`, would need to respect EditA.
    }};

    let COL_MUT: &Vec<_> = &table.cols.iter()
        .map(|x| if x.indexed { "EditA" } else { "MutA" })
        .map(i)
        .collect();

    let DELETED_ROW = quote_if(table.consistent, quote! {
        fn is_deleted(&self, idx: GenericRowId<Row>) -> bool {
            self._lock.free.get(&idx.to_usize()).is_some()
        }
    });
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
            // '#COL_MUT' is either MutA or EditA
            #(pub #COL_NAME: #COL_MUT<'u, #COL_TYPE>,)*
        }

        impl<'u> LockedTable for Read<'u> {
            type Row = Row;
            fn len(&self) -> usize { self.len() }
            #DELETED_ROW
        }
        impl<'u> LockedTable for Write<'u> {
            type Row = Row;
            fn len(&self) -> usize { self.len() }
            #DELETED_ROW
        }
    }};


    let RW_FUNCTIONS_CONSISTENT = quote! {
        /** Returns an iterator over each non-row in the table that is not marked for deletion.
         * (R/W) */
        pub fn iter(&self) -> ConsistentIter<Self> {
            self.range(self.row_range())
        }

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
    let RW_FUNCTIONS_INCONSISTENT = quote! {
        /** Returns an iterator over each row in the table. (R/W) */
        pub fn iter(&self) -> CheckedIter<Self> {
            self.range(self.row_range())
        }

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

        pub fn row_range(&self) -> RowRange<RowId> {
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
                    unsafe {
                        let i = row.check(self).to_usize();
                        #(
                            self.#COL_NAME.deref_mut().inner_mut().deleted(i);
                        )*
                        self.event_delete(i);
                    }
                }

                #(
                    /// `deleted` is a list of removed foreign keys.
                    pub fn #TRACK_IFC_REMOVAL(&mut self, deleted: &[usize]) {
                        for deleted_foreign in deleted {
                            type E = #IFC_ELEMENT;
                            let deleted_foreign = E::from_usize(*deleted_foreign);
                            let delete_range = (deleted_foreign, 0)..(deleted_foreign, ::std::usize::MAX);
                            loop {
                                let referenced_by = {
                                    let index = self.#IFC.deref().inner().get_index();
                                    if let Some((&(_foreign, local), &())) = index.range(delete_range.clone()).next() {
                                        local
                                    } else {
                                        break;
                                    }
                                };
                                let kill = RowId::from_usize(referenced_by);
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

    if table.sorted && !table.derive.clone {
        panic!("sorted tables must be clone");
    }
    out! { !table.immutable && table.derive.clone => ["merge functions"] {
        impl<'u> Write<'u> {
            pub fn retain<F: FnMut(&Self, RowId) -> bool>(&mut self, mut f: F) {
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
                    self.push(row);
                }
            }
        }
    };}
    out! { !table.immutable && table.derive.clone && !table.sorted => ["visit"] {
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
        !table.immutable && table.sorted => ["row pushing for sorted tables"] {
            impl<'u> Write<'u> {
                pub fn merge<IT: Iterator<Item=Row>, I: Into<AssertSorted<IT>>>(&mut self, rows: I)
                where IT: IntoIterator<Row>
                {
                    // This is actually a three-way merge. Joy!
                    // We have three things: the table, the rug, and the iter.
                    // We get 'next' by merging the rug and the iter.
                    // Whenever something gets bumped off the table, it is pushed onto the rug.
                    // The rug is sorted because the table is sorted.
                    let mut rug = VecDeque::new();
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
                                self.swap_out_row(i, next);
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
        !table.sorted => ["row pushing for unsorted tables"] {
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
                type E = #COL_TRACK_ELEMENTS;
                E::register_tracker(_universe, bx);
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

    if table.save && !table.derive.clone { panic!("#[save] requires #[row_derive(Clone)]"); }

    out! {
        // FIXME: Use Serde, and encode by columns instead.
        table.save => ["Save"] {
            use rustc_serialize::{Decoder, Decodable, Encoder, Encodable};

            impl<'u> Read<'u> {
                /// Row-based encoding.
                pub fn encode_rows<E: Encoder>(&self, e: &mut E) -> Result<(), E::Error> {
                    if !self._lock.skip_flush() { panic!("Encoding unflushed table!"); }
                    e.emit_struct(#TABLE_NAME_STR, 2, |e| {
                        e.emit_struct_field("free_list", 0, |e| self._lock.free.encode(e))?;
                        e.emit_struct_field("rows", 1, |e| e.emit_seq(self.len(), |e| {
                            for i in self.iter() {
                                let row = self.get_row(i);
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
                    caught
                }
            }
        };
    }

    Ok(())
}
