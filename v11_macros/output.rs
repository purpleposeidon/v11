use std::io::Write;

use quote::{Ident, Tokens};
use syntex_syntax::print::pprust as pp;

use super::table::Table;

/// Convert a string into a quote `Ident`.
fn i<S: AsRef<str>>(s: S) -> Ident {
    Ident::new(s.as_ref())
}

macro_rules! write_quote {
    ([$table:expr, $out:expr], $($args:tt)*) => {
        write_quote! {
            [$table, $out, ""]
            $($args:tt)*
        }
    };
    ([$table:expr, $out:expr, $section:expr], $($args:tt)*) => {
        let args = quote! { $($args)* };
        let buff = if $section.is_empty() {
            format!("{}\n", args)
        } else {
            format!("\n// {}\n{}\n\n", $section, args)
        };
        let buff = buff.replace("#TABLE_NAME", &$table.name);

        $out.write(buff.as_bytes())?;
    };
}

impl Table {
    fn if_tracking(&self, q: Tokens) -> Tokens {
        quote_if(self.track_changes, q)
    }
}

fn quote_if(b: bool, q: Tokens) -> Tokens {
    if b {
        q
    } else {
        quote! {}
    }
}

#[allow(non_snake_case)]
pub fn write_out<W: Write>(table: Table, mut out: W) -> ::std::io::Result<()> {
    // Info
    writeln!(out, "// Generated file. If you are debugging this output, put this in a module and uncomment this line:")?;
    writeln!(out, "// domain! {{ TABLE_DOMAIN }}\n\n")?;
    writeln!(out, "// Table config:")?;
    for line in format!("{:#?}", table).split('\n') {
        writeln!(out, "//   {}", line)?;
    }

    let str2i = |v: &Vec<String>| -> Vec<Ident> { v.iter().map(i).collect() };

    // "name": ["element"; "col_type"],
    use ::table::Col;
    let COL_NAME_STR: &Vec<_> = &table.cols.iter().map(|x| pp::ident_to_string(x.name)).collect();
    let COL_ELEMENT_STR: &Vec<_> = &table.cols.iter().map(|x| pp::ty_to_string(&*x.element)).collect();
    let COL_TYPE_STR: &Vec<_> = &table.cols.iter().map(|x| format!("ColWrapper<{}, RowId>", pp::ty_to_string(&*x.colty))).collect();
    let COL_ATTR: &Vec<_> = &table.cols.iter().map(|x: &Col| {
        let r = if let Some(a) = x.attrs.as_ref() {
            a.iter().map(pp::attr_to_string).map(|x| format!("{}\n", x)).collect()
        } else {
            "".to_string()
        };
        r
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
    write_quote! {
        [table, out, "Header"],

        use v11;
        use v11::Universe;
        use v11::intern::PBox;
        use v11::intern::{RefA, MutA};
        #[allow(unused_imports)]
        use v11::Event;
        use v11::tables::{GenericTable, GenericRowId, TableName, GetTableName, RowRange};
        use v11::columns::{TCol, ColWrapper};

        #[allow(unused_imports)]
        use v11::columns::{VecCol, BoolCol, SegCol};
        // Having them automatically imported is a reasonable convenience.

        pub const TABLE_NAME: TableName = TableName(#TABLE_NAME_STR);
        // TABLE_DOMAIN = super::#TABLE_DOMAIN
        pub const VERSION: usize = #TABLE_VERSION;

        #[allow(non_upper_case_globals)]
        mod column_format {
            #(pub const #COL_NAME: &'static str = #COL_FORMAT;)*
        }
    }

    let DERIVE_CLONE = quote_if(table.clone, quote! {
        #[derive(Clone)]
    });
    let DERIVE_ENCODING = quote_if(table.save, quote! { #[derive(RustcEncodable, RustcDecodable)] });
    let DERIVE_ENCODING_W = quote_if(table.save, quote! { #[derive(RustcEncodable)] });
    let DERIVE_DEBUG = quote_if(table.debug, quote! {
        #[derive(Debug)]
    });

    let ROW_ID_TYPE = i(&table.row_id);
    write_quote! {
        [table, out, "Indexing"],

        /// This is the type used to index into `#TABLE_NAME`'s columns.
        /// It is typed specifically for this table.
        pub type RowId = GenericRowId<#ROW_ID_TYPE, Row>;
        /// The internal index type, which specifies the maximum number of rows. It is controlled
        /// by `impl { RawType = u32 }`.
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
        fn fab(i: usize) -> RowId { at(i as #ROW_ID_TYPE) }
    };

    write_quote! {
        [table, out, "The `Row` struct"],

        /**
         * A structure holding a copy of each column's data. This is used to pass entire rows around through methods;
         * the actual table is column-based, so eg `read.column[index]` is the standard method of accessing rows.
         * */
        #DERIVE_CLONE
        #DERIVE_ENCODING
        #DERIVE_DEBUG
        // FIXME: How about RowDerive()?
        pub struct Row {
            #(#COL_ATTR pub #COL_NAME: #COL_ELEMENT,)*
        }
        impl GetTableName for Row {
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
    };

    let NEED_FLUSH = table.if_tracking(quote! {_needs_flush: bool,});
    let NEED_FLUSH_INIT = table.if_tracking(quote! { _needs_flush: true, });

    write_quote! {
        [table, out, "Table locks"],

        /**
         * The table, locked for writing.
         * */
        pub struct Write<'u> {
            _lock: ::std::sync::RwLockWriteGuard<'u, GenericTable>,
            #NEED_FLUSH
            #(pub #COL_NAME: MutA<'u, #COL_TYPE>,)*
        }
        /**
         * The table, locked for reading.
         * */
        pub struct Read<'u> {
            _lock: ::std::sync::RwLockReadGuard<'u, GenericTable>,
            #(pub #COL_NAME: RefA<'u, #COL_TYPE>,)*
        }
    };

    write_quote! {
        [table, out, "`context!` duck-type implementation"],

        // Hidden because `$table::read()` is shorter than `$table::Read::lock()`.
        impl<'u> Write<'u> {
            #[doc(hidden)] #[inline] pub fn lock(universe: &'u v11::Universe) -> Self { write(universe) }
            #[doc(hidden)] #[inline] pub fn lock_name() -> &'static str { concat!("mut ", #TABLE_NAME_STR) }
        }

        impl<'u> Read<'u> {
            #[doc(hidden)] #[inline] pub fn lock(universe: &'u v11::Universe) -> Self { read(universe) }
            #[doc(hdiden)] #[inline] pub fn lock_name() -> &'static str { concat!("ref ", #TABLE_NAME_STR) }
        }
    }

    let GET_ROW = quote_if(table.clone, quote! {
        /** Retrieves a structure containing a clone of the value in each column. (R/W) */
        pub fn get_row(&self, index: RowId) -> Row {
            Row {
                #(#COL_NAME: self.#COL_NAME2[index].clone(),)*
            }
        }
    });

    let DUMP_ROWS = quote_if(table.clone, quote! {
        /** Allocates a Vec filled with every Row in the table. (R/W) */
        pub fn dump(&self) -> Vec<Row> {
            let mut ret = Vec::with_capacity(self.len());
            for i in self.iter() {
                ret.push(self.get_row(i));
            }
            ret
        }
    });

    let RW_FUNCTIONS = quote! {
        /** Returns the number of rows in the table. (R/W) */
        // And assumes that the columns are all the same length.
        // But there shouldn't be any way to break that invariant.
        pub fn len(&self) -> usize {
            self.#COL0.len()
        }

        /// Gets the last `RowId`.
        pub fn last(&self) -> Option<RowId> {
            let r = self.len();
            if r == 0 {
                None
            } else {
                Some(fab(r - 1))
            }
        }

        /** Returns an iterator over each row in the table. (R/W) */
        pub fn iter(&self) -> v11::RowIdIterator<#ROW_ID_TYPE, Row> {
            v11::RowIdIterator::new(0, self.len() as #ROW_ID_TYPE)
        }

        /** Returns true if `i` is a valid RowId. */
        pub fn contains(&self, index: RowId) -> bool {
            index.to_usize() < self.len()
        }

        #GET_ROW

        /** Retrieves a structure containing a reference to each value in each column. (R/W) */
        pub fn get_row_ref(&self, index: RowId) -> RowRef {
            RowRef {
                #(#COL_NAME: &self.#COL_NAME2[index],)*
            }
        }

        #DUMP_ROWS

        /** Release this lock. (R/W) */
        pub fn close(self) {} // Highly sophisticated! :D
        // FIXME: Join
    };
    write_quote! {
        [table, out, "methods common to both Read and Write"],
        // We're only repeating ourselves twice here.

        impl<'u> Read<'u> {
            #RW_FUNCTIONS
        }
        impl<'u> Write<'u> {
            #RW_FUNCTIONS
        }
    }

    if table.track_changes {
        write_quote! {
            [table, out, "Change tracking"],

            impl<'a> Drop for Write<'a> {
                fn drop(&mut self) {
                    if self._needs_flush {
                        panic!("Changes to {} were not flushed", TABLE_NAME);
                    }
                }
            }

            use std::any::Any;

            /// Trackers receive events from tables when `flush` is called.
            /// The function can't lock the table.
            pub type Tracker = Box<FnMut(&Universe, &[Event<RowId>]) + Send + Sync>;

            /// Add a tracker.
            pub fn register_tracker(universe: &Universe, tracker: Tracker) {
                let mut gt = get_generic_table(universe).write().unwrap();
                let mut trackers = gt.trackers.write().unwrap();
                trackers.push(Box::new(tracker) as PBox);
            }

            impl<'a> Write<'a> {
                /// Allow the `Write` lock to be closed without flushing changes. Be careful!
                /// The changes need to be flushed eventually!
                pub fn noflush(&mut self) {
                    self._needs_flush = false;
                }

                /// Propagate all changes made thus far to the Trackers.
                pub fn flush(&mut self, universe: &Universe) {
                    {
                        let events = &self._events.data.data[..];
                        let gt = get_generic_table(universe).write().unwrap();
                        let mut trackers = gt.trackers.write().unwrap();
                        for tracker in trackers.iter_mut() {
                            let tracker: &mut Tracker = ::v11::intern::desync_box_mut(tracker).downcast_mut()
                                .expect("Tracker downcast failed");
                            tracker(universe, events);
                        }
                    }
                    self._events.clear();
                    self._needs_flush = false;
                }

                fn event(&mut self, event: Event<RowId>) {
                    self._events.push(event);
                    self._needs_flush = true;
                }
            }
        }
    }

    if table.track_changes {
        write_quote! {
            [table, out, "Extra drops (Drop for Write implemented above)"],
            /// Prevent moving out to improve `RefA` safety.
            impl<'u> Drop for Read<'u> {
                fn drop(&mut self) {}
            }
        }
    } else {
        write_quote! {
            [table, out, "Extra drops"],
            /// Prevent moving out to improve `RefA` safety.
            impl<'u> Drop for Read<'u> {
                fn drop(&mut self) {}
            }

            /// Prevent moving out to improve `MutA` safety.
            impl<'u> Drop for Write<'u> {
                fn drop(&mut self) {}
            }
        }
    }

    let EVENT_CLEAR = table.if_tracking(quote! { self.event(Event::ClearAll); });
    let EVENT_PUSH = table.if_tracking(quote! { self.event(Event::Create(rowid)); });

    write_quote! {
        [table, out, "safe mut methods"],
        impl<'u> Write<'u> {
            /** Prepare the table for insertion of a specific amount of data. `self.len()` is
             * unchanged. */
            pub fn reserve(&mut self, additional: usize) {
                #(self.#COL_NAME.data.reserve(additional);)*
            }

            /** Removes every row from the table. */
            pub fn clear(&mut self) {
                #(self.#COL_NAME.data.clear();)*
                #EVENT_CLEAR
            }

            /// Returns the RowId of the next row that would be inserted.
            pub fn next_pushed(&self) -> RowId {
                // FIXME: And if we've got a freelist?
                fab(self.len())
            }

            // Not really 'safe', but it's private.
            #[inline]
            fn push_end(&mut self, row: Row) -> RowId {
                #(self.#COL_NAME.data.push(row.#COL_NAME2);)*
                let rowid = self.next_pushed();
                #EVENT_PUSH
                rowid
            }

            #[inline]
            fn swap_row(&mut self, i: RowId, row: &mut Row) {
                use std::mem::swap;
                #(swap(&mut self.#COL_NAME[i], &mut row.#COL_NAME2);)*
            }
        }
    };

    if let Some(primary) = table.merge {
        write_quote! {
            [table, out, "mut methods for always-sorted table"],

            impl<'u> Write<'u> {
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
                            if i < self.len() && &self.#primary[i] <= next {
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
    } else {
        write_quote! {
            [table, out, "mut methods"],

            impl<'u> Write<'u> {
                #[inline]
                pub fn set_row(&mut self, index: RowId, row: Row) {
                    #(self.#COL_NAME[index] = row.#COL_NAME2;)*
                }

                // FIXME: freelist

                /** Populate the table with data from the provided iterator. */
                #[inline]
                pub fn push_all<I: ::std::iter::Iterator<Item=Row>>(&mut self, data: I) {
                    self.reserve(data.size_hint().0);
                    for row in data {
                        self.push_end(row);
                    }
                }

                /// Appends a single Row to the end of the table.
                /// Returns its RowId.
                #[inline]
                pub fn push(&mut self, row: Row) -> RowId {
                    self.push_end(row)
                }

                /// Push an 'array' of values. Contiguity is guaranteed.
                pub fn push_array<I>(&mut self, i: I) -> RowRange<RowId>
                where I: ExactSizeIterator<Item=Row>
                {
                    // This implementation doesn't need ExactSizeIterator, but future configurations
                    // (FreeList) will require it.
                    let start = self.next_pushed();
                    self.push_all(i);
                    let end = self.last().unwrap_or(start);
                    RowRange {
                        start: start,
                        end: end,
                    }
                }
            }
        };
    }

    write_quote! {
        [table, out, "Lock & Load"],

        use std::mem::transmute;
        use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, LockResult, TryLockResult};

        fn get_generic_table(universe: &v11::Universe) -> &RwLock<GenericTable> {
            let domain_id = TABLE_DOMAIN.get_id();
            universe.get_generic_table(domain_id, TABLE_NAME)
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
                _lock: _lock,
                #( #COL_NAME3: #COL_NAME4, )*
            }
        }

        /// Locks the table for reading.
        // We're too cool to be callling unwrap() all over the place.
        pub fn read(universe: &v11::Universe) -> Read {
            read_result(universe).unwrap()
        }

        /// This is equivalent to `RwLock::read`.
        pub fn read_result<'u>(universe: &'u v11::Universe) -> LockResult<Read<'u>> {
            let table = get_generic_table(universe).read();
            ::v11::intern::wrangle_lock::map_result(table, convert_read_guard)
        }

        pub fn try_read<'u>(universe: &'u v11::Universe) -> TryLockResult<Read<'u>> {
            let table = get_generic_table(universe).try_read();
            ::v11::intern::wrangle_lock::map_try_result(table, convert_read_guard)
        }




        fn convert_write_guard(mut _lock: RwLockWriteGuard<GenericTable>) -> Write {
            #(let #COL_NAME = {
                let got = _lock.get_column_mut::<#COL_TYPE2>(#COL_NAME_STR, column_format::#COL_NAME2);
                unsafe {
                    MutA::new(transmute(got))
                    // See comment about transmute in `convert_read_guard()`.
                }
            };)*
            Write {
                _lock: _lock,
                #NEED_FLUSH_INIT
                #( #COL_NAME3: #COL_NAME4, )*
            }
        }

        /// Locks the table for writing.
        pub fn write<'u>(universe: &'u v11::Universe) -> Write<'u> {
            write_result(universe).unwrap()
        }

        pub fn write_result<'u>(universe: &'u v11::Universe) -> LockResult<Write<'u>> {
            let table = get_generic_table(universe).write();
            ::v11::intern::wrangle_lock::map_result(table, convert_write_guard)
        }

        pub fn try_write<'u>(universe: &'u v11::Universe) -> TryLockResult<Write<'u>> {
            let table = get_generic_table(universe).try_write();
            ::v11::intern::wrangle_lock::map_try_result(table, convert_write_guard)
        }

        /// Register the table onto its domain.
        pub fn register() {
            let table = GenericTable::new(TABLE_DOMAIN, TABLE_NAME);
            let table = table #(.add_column(
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
            table.register();
        }
    };

    if table.save {
        write_quote! {
            [table, out, "Save"],

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

                /* -- This is kind of not possible to do due to funky bits in ColWrapper & BoolVec
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

    if !table.no_complex_mut {
        // These are baaasically incompatible w/ change tracking.
        // 1: The rug algorithm is already very complex.
        // 2: A simpler algorithm would do more allocation.
        // 3: Removing a single item generates O(n) updates; extremely messy.
        // (We could have a 'bulk-shift' event...)
        // (We could possibly expose these 'at your own risk', without actually doing the tracking,
        // if `track_changes` is on.)
        write_quote! {
            [table, out, "Complicated mut algorithms (non-tracking edition)"],

            /**
             * Ergonomics for calling `Write::visit` with a closure that does not add values.
             * 
             * `#TABLE_NAME.visit(|table, i| -> #TABLE_NAME::ClearVisit { … })`
             * */
            pub type ClearVisit = v11::Action<Row, v11::intern::VoidIter<Row>>;

            impl<'u> Write<'u> {
                /**
                 * Keep or discard rows according to the provided closure.
                 * If it returns `true`, then the row is kept.
                 * If it returns `false`, then the row is removed.
                 * */
                pub fn filter<F>(&mut self, mut closure: F)
                    where F: FnMut(&mut Write, RowId) -> bool
                {
                    self.visit(|wlock, rowid| -> ClearVisit {
                        if closure(wlock, rowid) {
                            v11::Action::Continue
                        } else {
                            v11::Action::Remove
                        }
                    });
                }

                /**
                 * Invokes the closure on every entry in the table. For each row, the closure can:
                 * *. remove that row
                 * *. modify that row
                 * *. do nothing to that row
                 * *. return an iterator to append an arbitrary number of rows
                 * It is to `Vec.retain`, but also allows insertion.
                 *
                 * This method can't insert rows into an empty table.
                 *
                 * If you want to remove & insert at the same time, you can do:
                 * ```
                 * #TABLE_NAME.set(row_id, iter.next().unwrap());
                 * return Action::Add(iter);
                 * 
                 * In addition to whatever the backing `TCol`s allocate, this function also allocates a
                 * `Vec` whose size is the number of inserted rows.
                 * ```
                 * */
                pub fn visit<IT, F>(&mut self, mut closure: F) // FIXME: This is dead. Kill it.
                    where IT: ::std::iter::Iterator<Item=Row>,
                           F: FnMut(&mut Write, RowId) -> v11::Action<Row, IT>
                {
                    // This is... complex.
                    // It's a "rug pushing" algorithm; you push a free-standing rug, and it makes a
                    // loop, and you can push the loop along until it hits the end & makes the rug
                    // longer. But also parts of the rug can be removed, causing the loop to collapse
                    // before reaching the end.

                    // This algorithm is probably close to maximum efficiency?
                    // About `number_of_insertions * sizeof(Row)` bytes of memory is allocated
                    // for internal temporary usage.

                    use std::collections::vec_deque::VecDeque;
                    // Temporary queue of rows that were displaced by inserts. In the middle of
                    // iteration, the length of this list is equal to the number of inserted rows.
                    // If this queue is not empty, then each visited row is pushed onto it,
                    // replaced with the popped front of the queue, and then that is what is
                    // actually visited. Should the end of the rowset be reached, this buffer is
                    // appended, and iteration continues.
                    let mut displaced_buffer: VecDeque<Row> = VecDeque::new();
                    // This is the read offset of the index, used when rows have been removed.
                    // It `rm_off > 0 && !displaced_buffer.is_empty()`, then rows from
                    // displaced_buffer need to be copied to the columns.
                    // It can be thought of as 'negative displacement_buffer size'.
                    let mut rm_off: usize = 0;
                    // Rows that have just been inserted must not be visited.
                    let mut skip = 0;

                    let mut index = 0usize;
                    fn flush_displaced(index: &mut usize,
                                       rm_off: &mut usize,
                                       all: &mut Write,
                                       displaced_buffer: &mut VecDeque<Row>) {
                        while *rm_off > 0 && !displaced_buffer.is_empty() {
                            all.set_row(fab(*index), displaced_buffer.pop_front().unwrap());
                            *index += 1;
                            *rm_off -= 1;
                        }
                    }

                    loop {
                        let len = self.len();
                        if index + rm_off >= len {
                            if displaced_buffer.is_empty() {
                                if rm_off > 0 {
                                    #(self.#COL_NAME.data.truncate(len - rm_off);)*
                                    rm_off = 0;
                                }
                                break;
                            }
                            flush_displaced(&mut index, &mut rm_off, self, &mut displaced_buffer); // how necessary?
                            while let Some(row) = displaced_buffer.pop_front() {
                                #(self.#COL_NAME.data.push(row.#COL_NAME2);)*
                                if skip > 0 {
                                    skip -= 1;
                                    index += 1;
                                }
                            }
                            continue; // 'goto top_of_block'.
                        }
                        if let Some(replacement) = displaced_buffer.pop_front() {
                            // Swap between '`here`' and the first displaced row.
                            // No garbage is produced.
                            displaced_buffer.push_back(self.get_row(fab(index)));
                            self.set_row(fab(index), replacement);
                            assert_eq!(rm_off, 0);
                        }
                        if rm_off > 0 {
                            // Move a row from the end of the garbage gap to the beginning.
                            // The front of the garbage gap is no longer garbage, and the back is
                            // now garbage.
                            let tmprow = self.get_row(fab(index + rm_off));
                            self.set_row(fab(index), tmprow);
                        }
                        // An invariant needs to be true at this point: self[index] is valid, not
                        // garbage data. What could make it garbage?
                        // This first loop, it's going to be fine.
                        // If remove has been used, then there are worries.
                        let action = if skip == 0 {
                            closure(self, fab(index))
                        } else {
                            skip -= 1;
                            v11::Action::Continue
                        };
                        match action {
                            v11::Action::Break => {
                                if rm_off == 0 && displaced_buffer.is_empty() {
                                    // Don't need to do anything
                                    break;
                                } else if !displaced_buffer.is_empty() {
                                    // simply stick 'em on the end
                                    while let Some(row) = displaced_buffer.pop_front() {
                                        #(self.#COL_NAME.data.push(row.#COL_NAME2);)*
                                    }
                                    // And we don't visit them.
                                    break;
                                } else if rm_off != 0 {
                                    // Trim.
                                    let start = index + 1;
                                    #(self.#COL_NAME.data.remove_slice(start..start+rm_off);)*
                                    rm_off = 0;
                                    break;
                                } else {
                                    // FIXME: #{} needs to be a thing. This little thing isn't
                                    // worth the hassle.
                                    // https://github.com/dtolnay/quote/issues/10
                                    // panic!("Shouldn't be here: rm_off={:?}, displaced_buffer={:?}", rm_off, displaced_buffer);
                                    panic!("Shouldn't be here: rm_off={:?}, displaced_buffer={:?}", rm_off, displaced_buffer.len());
                                }
                            },
                            v11::Action::Continue => { index += 1; },
                            v11::Action::Remove => {
                                match displaced_buffer.pop_front() {
                                    None => { rm_off += 1; },
                                    Some(row) => {
                                        self.set_row(fab(index), row);
                                        index += 1;
                                    },
                                }
                            },
                            v11::Action::Add(iter) => {
                                {
                                    // Must do a little dance; first item returned by the iterator
                                    // goes to the front of the queue, which is unnatural.
                                    displaced_buffer.reserve(iter.size_hint().0);
                                    let mut added = 0;
                                    for row in iter {
                                        displaced_buffer.push_back(row);
                                        added += 1;
                                    }
                                    skip += added;
                                    for _ in 0..added {
                                        let tmprow = displaced_buffer.pop_back().unwrap();
                                        displaced_buffer.push_front(tmprow);
                                    }
                                }
                                flush_displaced(&mut index, &mut rm_off, self, &mut displaced_buffer);
                                index += 1;
                            }
                        }
                    }
                    assert!(displaced_buffer.is_empty());
                    assert_eq!(rm_off, 0);
                }
            }
        };
    }

    if table.generic_sort || !table.sort_by.is_empty() {
        let PUB = if table.generic_sort {
            i("pub")
        } else {
            i("")
        };
        write_quote! {
            [table, out, "Generic sort"],

            use std::cmp::Ordering;
            impl<'u> Write<'u> {
                /// Sorts the table using the provided function.
                #PUB fn sort_with<C>(&mut self, mut compare: C)
                where C: FnMut(&Write<'u>, RowId, RowId) -> Ordering
                {
                    // We do this the lame way to avoid having to implement our own sorting
                    // algorithm.
                    // FIXME: Liberate std::collections::slice::merge_sort()
                    // Or maybe https://github.com/benashford/rust-lazysort ?
                    let mut indices: Vec<RawType> = (0..(self.len() as RawType)).collect();
                    {
                        indices.sort_by(|a: &RawType, b: &RawType| { compare(self, at(*a), at(*b)) });
                    }
                    self.apply_resort(indices);
                }

                fn apply_resort(&mut self, indices: Vec<RawType>) {
                    // Very inefficient!
                    let len = indices.len();
                    #({
                        let mut tmp = Vec::with_capacity(len);
                        let mut col = &mut self.#COL_NAME;
                        {
                            for i in &indices {
                                tmp.push(col[at(*i)]);
                                // This can have us jumping around a lot, making the cache sad.
                            }
                        }
                        col.data.clear();
                        col.data.append(&mut tmp);
                    })*
                }
            }
        }
    }

    for sort_key in &table.sort_by {
        let SORT_BY_COL = i(format!("sort_by_{}", sort_key));
        let SORTED_BY_COL = i(format!("sorted_by_{}", sort_key));
        let SORT_KEY = i(sort_key);

        write_quote! {
            [table, out, "sorting"],

            impl<'u> Write<'u> {
                /**
                 * Sort the table by the indicated key.
                 * */
                pub fn #SORT_BY_COL(&mut self) {
                    self.sort_with(|me: &Write<'u>, a: RowId, b: RowId| {
                        let col = &me.#SORT_KEY;
                        col[a].cmp(&col[b])
                    })
                }
            }
            /// Return the table locked for writing and sorted by $column.
            pub fn #SORTED_BY_COL(universe: &Universe) -> Write {
                let mut w = write(universe);
                w.#SORT_BY_COL();
                w
            }
        }
    }
    // FIXME: sorting change tracking

    if let Some(ref mod_code) = table.mod_code {
        writeln!(out, "// User code\n{}", mod_code)?;
    }

    Ok(())
}
