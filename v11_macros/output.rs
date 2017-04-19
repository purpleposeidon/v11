use std::io::Write;

use quote::{Ident, Tokens};
use syntex_syntax::print::pprust as pp;

use super::table::Table;

/// Convert a string into a quote `Ident`.
fn i<S: AsRef<str>>(s: S) -> Ident {
    Ident::new(s.as_ref())
}

macro_rules! write_quote {
    ([$table:expr, $out:expr, $section:expr] $($args:tt)*) => {
        let buff = format!("\n// {}\n{}\n\n", $section, quote! { $($args)* });
        let buff = buff.replace("#TABLE_NAME", &$table.name);

        $out.write(buff.as_bytes())?;
    };
}

#[allow(non_snake_case)]
pub fn write_out<W: Write>(table: Table, mut out: W) -> ::std::io::Result<()> {
    // Info
    writeln!(out, "// Generated file. Table config:")?;
    for line in format!("{:#?}", table).split('\n') {
        writeln!(out, "//   {}", line)?;
    }

    let str2i = |v: &Vec<String>| -> Vec<Ident> { v.iter().map(i).collect() };

    // "name": ["element"; "col_type"],
    use ::table::Col;
    let COL_NAME_STR: &Vec<_> = &table.cols.iter().map(|x| pp::ident_to_string(x.name)).collect();
    let COL_ELEMENT_STR: &Vec<_> = &table.cols.iter().map(|x| pp::ty_to_string(&*x.element)).collect();
    let COL_TYPE_STR: &Vec<_> = &table.cols.iter().map(|x| format!("ColWrapper<{}, RowId>", pp::ty_to_string(&*x.colty))).collect();
    let COL_ATTR: Vec<_> = table.cols.iter().map(|x: &Col| {
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
        [table, out, "Header"]

        use v11;
        use v11::intern::PBox;
        use v11::tables::{GenericTable, GenericRowId, TableName, GetTableName};
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

    let DERIVE_ENCODING: Tokens = {
        if table.serde {
            quote! { #[Serialize, Deserialize] }
        } else {
            quote! {}
        }
    };

    let DERIVE_DEBUG = if table.debug {
        quote! {
            #[derive(Debug)]
        }
    } else {
        quote! {}
    };

    let ROW_ID_TYPE = i(table.row_id);
    write_quote! {
        [table, out, "Indexes"]

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

        // FIXME: Can we change this to 'unsafe'? What uses this that isn't covered above?
        /// Creates an index into the `i`th row.
        pub fn at(i: #ROW_ID_TYPE) -> RowId { RowId::new(i) }
        fn fab(i: usize) -> RowId { at(i as #ROW_ID_TYPE) }
    };

    write_quote! {
        [table, out, "The `Row` struct"]

        /**
         * A structure holding a row's data. This is used to pass rows around through methods;
         * the actual table is column-based, so eg `read.column[index]` is the standard method
         * of accessing rows.
         * */
        #[derive(PartialEq, Copy, Clone)]
        #DERIVE_ENCODING
        #DERIVE_DEBUG
        // FIXME: Do we need PartialEq?
        // FIXME: How about RowDerive()?
        pub struct Row {
            #(#COL_ATTR pub #COL_NAME: #COL_ELEMENT,)*
        }
        impl GetTableName for Row {
            fn get_name() -> TableName { TABLE_NAME }
        }
    };

    write_quote! {
        [table, out, "Table locks"]

        /**
         * The table, locked for writing.
         * */
        pub struct Write<'u> {
            _lock: ::std::sync::RwLockWriteGuard<'u, GenericTable>,
            #(pub #COL_NAME: &'u mut #COL_TYPE,)*
        }

        /**
         * The table, locked for reading.
         * */
        pub struct Read<'u> {
            _lock: ::std::sync::RwLockReadGuard<'u, GenericTable>,
            #(pub #COL_NAME: &'u #COL_TYPE,)*
        }
    };

    let RW_FUNCTIONS = quote! {
        /** Returns the number of rows in the table. (R/W) */
        // And assumes that the columns are all the same length.
        // But there shouldn't be any way to break that invariant.
        pub fn rows(&self) -> usize {
            self.#COL0.len()
        }

        /** Returns an iterator over each row in the table. (R/W) */
        pub fn range(&self) -> v11::RowIdIterator<#ROW_ID_TYPE, Row> {
            v11::RowIdIterator::new(0, self.rows() as #ROW_ID_TYPE)
        }

        /** Returns true if `i` is a valid RowId. */
        pub fn contains(&self, index: RowId) -> bool {
            index.to_usize() < self.rows()
        }

        /** Retrieves a structure containing a copy of the value in each column. (R/W) */
        pub fn get_row(&self, index: RowId) -> Row {
            Row {
                #(#COL_NAME: self.#COL_NAME2[index],)*
            }
        }

        /** Allocates a Vec filled with every Row in the table. (R/W) */
        pub fn dump(&self) -> Vec<Row> {
            let mut ret = Vec::with_capacity(self.rows());
            for i in self.range() {
                ret.push(self.get_row(i));
            }
            ret
        }

        /** Release this lock. (R/W) */
        pub fn close(self) {} // Highly sophisticated! :D
        // FIXME: Join
    };
    write_quote! {
        [table, out, "methods common to both Read and Write"]
        // FIXME: It'd kinda be nice to not have to do it this ugly way...
        // Could we have some kind of CommonLock?
        // It could be difficult to do that in an ergonomic way tho.
        
        impl<'u> Read<'u> {
            #RW_FUNCTIONS
        }
        impl<'u> Write<'u> {
            #RW_FUNCTIONS
        }
    }

    write_quote! {
        [table, out, "mut methods"]

        impl<'u> Write<'u> {
            /** Prepare the table for insertion of a specific amount of data. `self.rows()` is
             * unchanged. */
            pub fn reserve(&mut self, additional: usize) {
                #(self.#COL_NAME.data.reserve(additional);)*
            }

            /** Removes every row from the table. */
            pub fn clear(&mut self) {
                #(self.#COL_NAME.data.clear();)*
            }

            pub fn set_row(&mut self, index: RowId, row: Row) {
                #(self.#COL_NAME[index] = row.#COL_NAME2;)*
            }

            /** Populate the table with data from the provided iterator. */
            pub fn push_all<I: ::std::iter::Iterator<Item=Row>>(&mut self, data: I) {
                self.reserve(data.size_hint().0);
                for row in data {
                    self.push1(row);
                }
            }

            /// Appends a single Row to the end of the table.
            /// Returns its RowId.
            pub fn push(&mut self, row: Row) -> RowId {
                self.push1(row);
                fab(self.rows() - 1)
            }

            fn push1(&mut self, row: Row) {
                #(self.#COL_NAME.data.push(row.#COL_NAME2);)*
            }
        }
    };

    write_quote! {
        [table, out, "Lock & Load"]

        use std::mem::transmute;
        use std::sync::RwLock;

        fn get_generic_table(universe: &v11::Universe) -> &RwLock<GenericTable> {
            let domain_id = TABLE_DOMAIN.get_id();
            universe.get_generic_table(domain_id, TABLE_NAME)
        }

        /// Locks the table for reading.
        pub fn read(universe: &v11::Universe) -> Read {
            let table = get_generic_table(universe);
            let _lock = table.read().unwrap();
            #(let #COL_NAME = {
                let got = _lock.get_column::<#COL_TYPE2>(#COL_NAME_STR, column_format::#COL_NAME2);
                unsafe {
                    transmute(got)
                }
            };)*
            Read {
                _lock: _lock,
                #( #COL_NAME3: #COL_NAME4, )*
            }
        }

        /// Locks the table for writing.
        pub fn write(universe: &v11::Universe) -> Write {
            let table = get_generic_table(universe);
            let mut _lock = table.write().unwrap();
            #(let #COL_NAME = {
                let got = _lock.get_column_mut::<#COL_TYPE2>(#COL_NAME_STR, column_format::#COL_NAME2);
                unsafe {
                    transmute(got)
                }
            };)*
            Write {
                _lock: _lock,
                #( #COL_NAME3: #COL_NAME4, )*
            }
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

    if table.serde {
        write_quote! {
            [table, out, "Serde"]

            impl<'u> Read<'u> {
                /// Row-based encoding.
                pub fn encode_rows<E: Encoder>(&mut self, e: &mut E) -> Result<(), E::Error> {
                    let rows = self.rows();
                    e.emit_u64(rows as u64)?;
                    for i in self.range() {
                        let row = self.get_row(i);
                        row.encode(e)?;
                    }
                    Ok(())
                }

                /// Column-based encoding.
                pub fn encode_columns<E: Encoder>(&mut self, e: &mut E) -> Result<(), E::Error> {
                    #(self.#COL_NAME.data.encode(e)?;)*
                    Ok(())
                }
            }

            impl<'u> Write<'u> {
                /// Row-based decoding. Clears the table before reading, and clears the table if
                /// there is an error.
                pub fn decode_rows<D: Decoder>(&mut self, d: &mut D) -> Result<(), D::Error> {
                    self.clear();
                    let rows = d.read_u64()? as usize;
                    self.reserve(rows);
                    for _ in 0..rows {
                        let row = Row::decode(d)?;
                        #(self.#COL_NAME.data.push(row.#COL_NAME2);)*
                    }
                    Ok(())
                }

                /// Column-based decoding. Clears the table before reading, and clears the table if
                /// there is an error.
                pub fn decode_columns<D: Decoder>(&mut self, d: &mut D) -> Result<(), D::Error> {
                    self.clear();
                    let decode = || {
                        #(self.#COL_NAME.data.decode(d)?;)*
                    };
                    if decode.is_err() {
                        self.clear();
                    }
                    decode
                }
            }
        };
    }

    write_quote! {
        [table, out, "Complicated mut algorithms"]

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
            pub fn visit<IT, F>(&mut self, mut closure: F)
                where IT: ::std::iter::Iterator<Item=Row>,
                       F: FnMut(&mut Write, RowId) -> v11::Action<Row, IT>
            {
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
                    let len = self.rows();
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
                        continue;
                    }
                    if let Some(replacement) = displaced_buffer.pop_front() {
                        // Swap between '`here`' and the first displaced row.
                        // No garbage is produced.
                        displaced_buffer.push_back(self.get_row(fab(index)));
                        self.set_row(fab(index), replacement);
                        assert!(rm_off == 0);
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
                                for row in displaced_buffer.iter() {
                                    #(self.#COL_NAME.data.push(row.#COL_NAME2);)*
                                }
                                displaced_buffer.clear();
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
                assert!(rm_off == 0);
            }
        }
    };

    if table.generic_sort || !table.sort_by.is_empty() {
        let PUB = if table.generic_sort {
            i("pub")
        } else {
            i("")
        };
        write_quote! {
            [table, out, "Generic sort"]

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
                    let mut indices: Vec<RawType> = (0..(self.rows() as RawType)).collect();
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
                            for i in indices.iter() {
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

    for sort_key in table.sort_by.iter() {
        let SORT_BY_COL = i(format!("sort_by_{}", sort_key));
        let SORTED_BY_COL = i(format!("sorted_by_{}", sort_key));
        let SORT_KEY = i(sort_key);

        write_quote! {
            [table, out, "sorting"]

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

    if let Some(ref mod_code) = table.mod_code {
        writeln!(out, "// User code\n{}", mod_code)?;
    }

    Ok(())
}
