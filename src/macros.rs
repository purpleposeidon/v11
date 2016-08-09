#![macro_use]

/**
 * This macro creates a table containing the implementation of the table.
 *
 * ```ignored
 * mod table_use {
 *     // Custom type definitions.
 *     pub use ::column_type_1;
 *     pub use parent::column_type_2;
 * }
 * use self::table_use::*; /* recommended */
 * table! {
 *      [name_of_table],
 *      column_name_1: column_type_1,
 *      column_name_2: column_type_2,
 *      /* … */
 * }
 * ```
 *
 * This generates a module named `name_of_table` with `load()` and `register()` functions.
 * Since non-standard types are unpleasant to refer to, a "mod table_use" should be created at the
 * same depth as the `table!` invocation.
 *
 * Table and column names must be valid rust identifiers, and must match the regex
 * "[A-Za-z][A-Za-z_0-9]*".
 *
 * Column types must implement `Storable`.
 *
 * Table sorting is done by the value in the first column.
 *
 */
#[macro_export]
macro_rules! table {
    (
        [$TN:ident],
        $HEAD_COL_NAME:ident: $HEAD_COL_TYPE:ty,
        $($CN:ident: $CT:ty,)* /* trailing comma required */
    ) => {
        pub mod $TN {
            /* public? Mmm. */
            table_impl! {
                [$TN, head = $HEAD_COL_NAME],
                $HEAD_COL_NAME: ($HEAD_COL_TYPE, Col<$HEAD_COL_TYPE>),
                $($CN: ($CT, Col<$CT>),)*
            }
        }
    };
}

macro_rules! table_impl {
    (
        [$TN:ident, head = $HEAD:ident],
        $($CN:ident: ($CT:ty, $DCT:ty),)*
    ) => {
        use std::iter::Iterator;
        use std::marker::PhantomData;
        use std::sync::{RwLockReadGuard, RwLockWriteGuard};

        use $crate::{Universe, OpaqueIndex, Action, RowIndexIterator, Col};
        use $crate::intern::{GenericTable, VoidIter};

        #[allow(unused_imports)]
        use super::table_use::*;

        /**
         * A structure holding a row's data. This is used to pass rows around through methods;
         * the actual table is column-based, so eg `read.column[index]` is the standard method
         * of accessing rows.
         * */
        #[derive(Debug, PartialEq, Copy, Clone)]
        pub struct Row {
            $(pub $CN: $CT,)*
        }

        fn fab(i: usize) -> OpaqueIndex<Row> {
            unsafe { OpaqueIndex::fabricate(i) }
        }

        /**
         * The table, locked for writing.
         * */
        pub struct Write<'u> {
            _lock: RwLockWriteGuard<'u, GenericTable>,
            _is_sorted: &'u mut bool,
            $(pub $CN: &'u mut $DCT,)*
        }
        impl<'u> Write<'u> {
            /** Returns the number of rows in the table.
             * (And assumes that the columns are all the same length.)
             * */
            pub fn rows(&self) -> usize {
                self.$HEAD.len()
            }

            /// Retrieves a structure containing a copy of the value in each column.
            pub fn row(&self, i: OpaqueIndex<Row>) -> Row {
                Row {
                    $($CN: self.$CN[i],)*
                }
            }

            pub fn range(&self) -> RowIndexIterator<Row> {
                RowIndexIterator {
                    i: 0,
                    end: self.rows(),
                    rt: PhantomData,
                }
            }

            fn set(&mut self, index: usize, row: Row) {
                // why not s/usize/OpaqueIndex & pub?
                let index = fab(index);
                $(self.$CN[index] = row.$CN;)*
            }

            fn get(&self, index: usize) -> Row {
                let index = fab(index);
                Row {
                    $($CN: self.$CN[index],)*
                }
            }

            /** Populate the table with data from the provided iterator. */
            pub fn push_all<I: Iterator<Item=Row>>(&mut self, data: I) {
                for row in data {
                    $(self.$CN.push(row.$CN);)*
                }
                *self._is_sorted = false;
                // Could set _is_sorted only if the values we push actually cause it to become
                // unsorted.
            }

            /// Appends a single Row to the end of the table.
            pub fn push(&mut self, data: Row) {
                self.push_all(Some(data).into_iter());
            }

            /** Removes every row from the table. */
            pub fn clear(&mut self) {
                $(self.$CN.clear();)*
                *self._is_sorted = true;
            }

            /**
             * Invokes the closure on every entry in the table. For each entry, the closure can
             * remove, modify, and insert arbitrary numbers of rows.
             *
             * This function of course can not be used to insert entries into an empty table.
             *
             * Similar to `Vec.retain`, but also allows insertion.
             *
             * If you want to visit without inserting, you will still need to provide a type for an
             * un-used iterator.
             * */
            pub fn visit<IT, F>(&mut self, mut closure: F)
                where IT: Iterator<Item=Row>,
                       F: FnMut(&mut Write, OpaqueIndex<Row>) -> Action<Row, IT> {
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
                        all.set(*index, displaced_buffer.pop_front().unwrap());
                        *index += 1;
                        *rm_off -= 1;
                    }
                }

                loop {
                    let len = self.rows();
                    if index + rm_off >= len {
                        if displaced_buffer.is_empty() {
                            if rm_off > 0 {
                                $(self.$CN.truncate(len - rm_off);)*
                                rm_off = 0;
                            }
                            break;
                        }
                        flush_displaced(&mut index, &mut rm_off, self, &mut displaced_buffer); // how necessary?
                        while let Some(row) = displaced_buffer.pop_front() {
                            $(self.$CN.push(row.$CN);)*
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
                        displaced_buffer.push_back(self.get(index));
                        self.set(index, replacement);
                        assert!(rm_off == 0);
                    }
                    if rm_off > 0 {
                        // Move a row from the end of the garbage gap to the beginning.
                        // The front of the garbage gap is no longer garbage, and the back is
                        // now garbage.
                        let tmprow = self.get(index + rm_off);
                        self.set(index, tmprow);
                    }
                    // An invariant needs to be true at this point: self[index] is valid, not
                    // garbage data. What could make it garbage?
                    // This first loop, it's going to be fine.
                    // If remove has been used, then there are worries.
                    let action = if skip == 0 {
                        closure(self, OpaqueIndex::new(index))
                    } else {
                        skip -= 1;
                        Action::Continue
                    };
                    match action {
                        Action::Break => {
                            if rm_off == 0 && displaced_buffer.is_empty() {
                                // Don't need to do anything
                                break;
                            } else if !displaced_buffer.is_empty() {
                                // simply stick 'em on the end
                                for row in displaced_buffer.iter() {
                                    $(self.$CN.push(row.$CN);)*
                                }
                                displaced_buffer.clear();
                                // And we don't visit them.
                                break;
                            } else if rm_off != 0 {
                                // Trim.
                                let start = index + 1;
                                $(self.$CN.drain(start..start+rm_off);)*
                                rm_off = 0;
                                break;
                            } else {
                                panic!("Shouldn't be here: rm_off={:?}, displaced_buffer={:?}", rm_off, displaced_buffer);
                            }
                        },
                        Action::Continue => { index += 1; },
                        Action::Remove => {
                            match displaced_buffer.pop_front() {
                                None => { rm_off += 1; },
                                Some(row) => {
                                    self.set(index, row);
                                    index += 1;
                                },
                            }
                        },
                        Action::Add(iter) => {
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
                                if added > 0 { *self._is_sorted = false; }
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

            /**
             * Sorts by the first key only. If you wanted to sort by multiple columns, you will
             * have to pack them into a tuple in the first column.
             * */
            pub fn sort(&mut self) {
                if *self._is_sorted { return; }

                // We do this the lame way to avoid having to implement our own sorting
                // algorithm.
                // TODO: Lots of work implementing custom sorting algorithms for various SOA
                // structures.
                let indices = {
                    let mut indices: Vec<usize> = (0..self.rows()).collect();
                    indices.sort_by_key(|i| { self.$HEAD[fab(*i)] });
                    indices
                };
                $({
                    let mut tmp = Vec::with_capacity(indices.len());
                    {
                        for i in indices.iter() {
                            tmp.push(self.$CN[fab(*i)]);
                            // This has us potentially jumping around a lot, altho of course
                            // often times the table will already be at least mostly-sorted.
                            // (Well, it'll tend to be mostly sorted already, right?)
                            // So we'll operate per-column, rather than over all columns, at a
                            // time to maximize the chances of a cache hit.
                            // (Well, maybe it'd be faster the other way???? Fancy Intel cache
                            // prediction?)
                        }
                    }
                    self.$CN.clear();
                    self.$CN.append(&mut tmp);
                })*
                *self._is_sorted = true;
            }
        }

        /**
         * If calling `Write.visit` with a closure that does not `Add` values,
         * there's no reasonable way to the type of the iterator that is never
         * used... until now!
         *
         * `$TN.visit(|table, i| -> $TN::ClearVisit { … })`
         * */
        pub type ClearVisit = Action<Row, VoidIter<Row>>;

        /**
         * The table, locked for reading.
         * */
        pub struct Read<'u> {
            _lock: RwLockReadGuard<'u, GenericTable>,
            _is_sorted: &'u bool,
            $(pub $CN: &'u $DCT,)*
        }
        impl<'u> Read<'u> {
            /** Returns the number of rows in the table.
             * (And assumes that the columns are all the same length.)
             * */
            pub fn rows(&self) -> usize {
                self.$HEAD.len()
            }

            pub fn range(&self) -> RowIndexIterator<Row> {
                RowIndexIterator {
                    i: 0,
                    end: self.rows(),
                    rt: PhantomData,
                }
            }

            /// Retrieves a structure containing a copy of the value in each column.
            pub fn row(&self, i: OpaqueIndex<Row>) -> Row {
                Row {
                    $($CN: self.$CN[i],)*
                }
            }

            // TODO: iter()
            // TODO: Join
        }

        /// Locks the table for reading (with the default name).
        pub fn read(universe: &Universe) -> Read { default().read(universe) }
        /// Locks the table for writing (with the default name).
        pub fn write(universe: &Universe) -> Write { default().write(universe) }
        /// Sorts the table, and then re-locks for writing (with the default name).
        pub fn sorted(universe: &Universe) -> Read { default().sorted(universe) }

        /**
         * Creates a TableLoader with the default table name, $TN.
         * */
        pub fn default() -> TableLoader<'static> {
            with_name(stringify!($TN))
        }

        /**
         * Creates a TableLoader with a custom table name.
         * (But beware that creating arbitrary numbers of identical tables runs contrary to the
         * spirit of Data Driven Programming.)
         * */
        pub fn with_name(name: &str) -> TableLoader {
            TableLoader {
                name: name,
            }
        }

        /**
         * Use `$TN::default()` or `$TN::with_name(&str)` to construct this builder.
         * */
        pub struct TableLoader<'s> {
            // Rust doesn't have default parameters! :>
            name: &'s str,
        }
        impl<'s> TableLoader<'s> {
            /** Locks the table for reading.
             * */
            pub fn read<'u>(&self, universe: &'u Universe) -> Read<'u> {
                let table = universe.get_generic_table(self.name);
                let _lock = table.read().unwrap();
                use std::mem::transmute;
                let _is_sorted = unsafe { transmute(&_lock.is_sorted) };
                $( let $CN = unsafe { transmute(_lock.get_column::<$CT>(stringify!($CN), stringify!($CT))) }; )*
                Read {
                    _lock: _lock,
                    _is_sorted: _is_sorted,
                    $( $CN: $CN, )*
                }
            }

            /**
             * Locks the table for writing.
             * */
            pub fn write<'u>(&self, universe: &'u Universe) -> Write<'u> {
                let table = universe.get_generic_table(self.name);
                let mut _lock = table.write().unwrap();
                use std::mem::transmute;
                let mut _is_sorted = unsafe { transmute(&mut _lock.is_sorted) };
                $( let $CN = unsafe { transmute(_lock.get_column_mut::<$CT>(stringify!($CN), stringify!($CT))) }; )*
                Write {
                    _lock: _lock,
                    _is_sorted: _is_sorted,
                    $( $CN: $CN, )*
                }
            }

            /**
             * Locks the table for reading, but first sorts it if necessary.
             * */
            pub fn sorted<'u>(&self, universe: &'u Universe) -> Read<'u> {
                for _ in 0..4 {
                    {
                        let ret = self.read(universe);
                        if *ret._is_sorted {
                            return ret;
                        }
                    }
                    {
                        let mut tab = self.write(universe);
                        tab.sort();
                    }
                }
                panic!("table {} isn't staying sorted!", self.name);
                // I can see how this could actually be a problem.
                // Perhaps if we fail, say, 8 times, we could do some trickery with threads to
                // ensure that our write is guaranteed to be next:
                //      spawn thread
                //      thread: lock for writing
                //      lock for reading (and must wait for the thread)
                //      thread: sort
                //      thread: finish, releasing lock
                //      table should be sorted & locked for reading
                //      maybe
                // Or forcefully stop all other threads.
                // (Actually what would be best is a way to convert a RwLockWriteGuard into a
                // RwLockReadGuard. Perhaps Rust could add something for converting between lock
                // types?)
            }

            /** Registers the table. */
            pub fn register(&self, universe: &mut Universe) {
                let table = GenericTable::new(self.name);
                $(let table = table.add_column::<$CT>(stringify!($CN), stringify!($CT), Col::new());)*
                universe.add_table(table);
            }
        }
    };
}

