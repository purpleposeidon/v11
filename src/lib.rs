//! A column-based in-memory database for [Data-Oriented
//! Programming](http://www.dataorienteddesign.com/dodmain/).
//! The tables and columns are dynamically described, but user-code interacts with them using
//! static dispatch. This requires the use of a macro to create a statically dispatched view of the
//! table, which does cause duplication of code & schema. However, this allows a large program to
//! dynamically load/hotswap libraries, and the crate for each library can have minimal
//! dependencies, and so can compile faster.
// I'm almost certain that's how that works.
//! 

extern crate rustc_serialize;
extern crate itertools;
extern crate joinkit;

use std::any::Any;
use std::sync::*;
use std::collections::HashMap;
use rustc_serialize::{Decodable, Encodable};

/**
 * Trait that all storable types must implement.
 *
 * Types that implement this trait should also not implement `Drop`, although this is not yet
 * expressable, and is not presently required.
 * */
// We really do want to be able to store floats, which means that we can't use proper Eq or
// PartialEq...
pub trait Storable : Default + Sync + Copy + Sized + Decodable + Encodable + PartialOrd /* + !Drop */ { }

macro_rules! storables_table {
    ($($T:ty),*,) => {
        $(impl Storable for $T {})*
    }
}
storables_table! {
    i8, i16, i32, i64,
    u8, u16, u32, u64,
    isize, usize,
    f32, f64,
    bool, char,
    // [char; 4], [char; 8], [char; 16], [char; 32], [char; 64],
}


/**
 * A context object that should be passed around everywhere.
 * */
pub struct Universe {
    tables: HashMap<String, RwLock<GenericTable>>,
    /* TODO: Properties. */
}
impl Universe {
    /** Create a new Universe. */
    pub fn new() -> Universe {
        Universe {
            tables: HashMap::new(),
        }
    }

    /**
     * Returns a string describing all the tables in the Universe. (But does not include their
     * contents.)
     * */
    pub fn info(&self) -> String {
        self.tables.iter().map(|(_, table)| {
            table.read().unwrap().info()
        }).collect::<Vec<String>>().join(" ")
    }
}
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
            None => panic!("Table {} does not exist", name),
            Some(t) => t.clone(),
        }
    }
}

/// A table held by `Universe`. Its information is used to create populate concrete tables.
pub struct GenericTable {
    pub name: String,
    pub is_sorted: bool,
    pub columns: Vec<GenericColumn>,
}
impl GenericTable {
    pub fn new(name: &str) -> GenericTable {
        check_name(name);
        GenericTable {
            name: name.to_string(),
            columns: Vec::new(),
            is_sorted: true,
        }
    }

    pub fn add_column<D, C>(mut self, name: &str, type_name: &'static str, inst: C) -> Self
    where D: Any + Storable + 'static,
          C: ConcreteCol<D> + Any {
        // Why is the 'static necessary??? Does it refer to the vtable or something?
        check_name(name);
        for c in self.columns.iter() {
            if c.name == name {
                panic!("Duplicate column name {}", name);
            }
        }
        self.columns.push(GenericColumn {
            name: name.to_string(),
            data: Box::new(inst) as Box<Any>,
            stored_type_name: type_name.to_string(),
        });
        self
    }

    pub fn get_column<D, C>(&self, name: &str, type_name: &str) -> &C
    where D: Any + Storable + 'static,
          C: ConcreteCol<D> + Any {
        let c = self.columns.iter().filter(|c| c.name == name).next().unwrap_or_else(|| {
            panic!("Table {} doesn't have a {} column.", self.name, name);
        });
        if c.stored_type_name != type_name { panic!("Column {}:{} has datatype {}, not {}", self.name, name, c.stored_type_name, type_name); }
        println!("get_column: {} {}", name, type_name);
        c.data.downcast_ref().unwrap()
    }

    pub fn get_column_mut<D: Any + Storable + 'static, C: ConcreteCol<D> + Any>(&mut self, name: &str, type_name: &str) -> &mut C {
        let my_name = &self.name;
        let c = self.columns.iter_mut().filter(|c| c.name == name).next().unwrap_or_else(|| {
            panic!("Table {} doesn't have a {} column.", my_name, name);
        });
        if c.stored_type_name != type_name { panic!("Column {}:{} has datatype {}, not {}", self.name, name, c.stored_type_name, type_name); }
        println!("get_column_mut: {} {}", name, type_name);
        c.data.downcast_mut().unwrap()
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

    fn info(&self) -> String {
        let mut ret = format!("{}:", self.name);
        for col in self.columns.iter() {
            ret.push_str(&format!(" {}:[{}]", col.name, col.stored_type_name));
        }
        ret
    }
}

pub struct GenericColumn {
    name: String,
    stored_type_name: String,
    data: Box<Any>,
}

pub trait ConcreteCol<D: Storable> {
    fn get(&self, index: usize) -> &D;
    fn get_mut(&mut self, index: usize) -> &mut D;
}
impl<D: Storable> ::std::ops::Index<OpaqueIndex> for ConcreteCol<D> {
    type Output = D;
    fn index(&self, index: OpaqueIndex) -> &D { self.get(index.0) }
}
impl<D: Storable> ::std::ops::IndexMut<OpaqueIndex> for ConcreteCol<D> {
    fn index_mut(&mut self, index: OpaqueIndex) -> &mut D { self.get_mut(index.0) }
}

impl<D: Storable> ConcreteCol<D> for Vec<D> {
    fn get<'a>(&'a self, index: usize) -> &'a D {
        use std::ops::Index;
        Vec::<D>::index(self, index)
    }
    fn get_mut<'a>(&'a mut self, index: usize) -> &'a mut D {
        use std::ops::IndexMut;
        Vec::<D>::index_mut(self, index)
    }
}





/** An index into a table. It is a bad idea to be dependent on the contents of this value, as
* tables may be sorted asynchronously/you would have to keep things updated, etc. Consider using an
* explicit index column.
*
* TODO: Add a lifetime & table-type parameter to guarantee more safety.
* */
pub struct OpaqueIndex(usize);
impl OpaqueIndex {
    /** Get the underlying index value. */
    pub unsafe fn extricate(&self) -> usize {
        self.0
    }

    /** Create a phoney OpaqueIndex. */
    pub unsafe fn fabricate(i: usize) -> OpaqueIndex {
        OpaqueIndex(i)
    }
}



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
                $HEAD_COL_NAME: ($HEAD_COL_TYPE, Vec<$HEAD_COL_TYPE>),
                $($CN: ($CT, Vec<$CT>),)*
            }
        }
    };
}

macro_rules! table_impl {
    (
        [$TN:ident, head = $HEAD:ident],
        $($CN:ident: ($CT:ty, $DCT:ty),)*
    ) => {
        use $crate::{Universe, GenericTable, OpaqueIndex, Action};
        use std::iter::Iterator;
        use std::sync::*;
        use std::ops::Range;
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

            pub fn range(&self) -> Range<usize> {
                0..self.rows()
            }

            fn set(&mut self, index: usize, row: Row) {
                // why not s/usize/OpaqueIndex & pub?
                $(self.$CN[index] = row.$CN;)*
            }

            fn get(&self, index: usize) -> Row {
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
             * */
            pub fn visit<IT, F>(&mut self, mut closure: F)
                where IT: Iterator<Item=Row>,
                       F: FnMut(&mut Write, OpaqueIndex) -> Action<Row, IT> {
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
                        for row in displaced_buffer.iter() {
                            $(self.$CN.push(row.$CN);)*
                        }
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
                        closure(self, OpaqueIndex(index))
                    } else {
                        skip -= 1;
                        Action::Next
                    };
                    match action {
                        Action::Stop => {
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
                        Action::Next => { index += 1; },
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
                    let mut indices: Vec<usize> = self.range().collect();
                    indices.sort_by_key(|i| { self.$HEAD[*i] });
                    indices
                };
                $({
                    let mut tmp = Vec::with_capacity(indices.len());
                    {
                        for i in indices.iter() {
                            tmp.push(self.$CN[*i]);
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

            pub fn range(&self) -> Range<usize> {
                0..self.rows()
            }

            // TODO: iter()
            // TODO: Join
        }

        /// Locks the table for reading (with the default name).
        pub fn read(universe: &Universe) -> Read { default().read(universe) }
        /// Locks the table for writing (with the default name).
        pub fn write(universe: &Universe) -> Write { default().write(universe) }

        /**
         * Creates a TableLoader with the default table name, $TN.
         * */
        pub fn default() -> TableLoader<'static> {
            with_name(stringify!($TN))
        }

        /**
         * Creates a TableLoader with a custom table name.
         * (Beware that creating arbitrary numbers of identical tables runs against the spirit of
         * Data Driven Programming.)
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
                $( let $CN = unsafe { transmute(_lock.get_column::<$CT, $DCT>(stringify!($CN), stringify!($CT))) }; )*
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
                $( let $CN = unsafe { transmute(_lock.get_column_mut::<$CT, $DCT>(stringify!($CN), stringify!($CT))) }; )*
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
                // RwLockReadGuard.)
            }

            /** Registers the table. */
            pub fn register(&self, universe: &mut Universe) {
                let table = GenericTable::new(self.name);
                $(let table = table.add_column::<$CT, $DCT>(stringify!($CN), stringify!($CT), Vec::new());)*
                universe.add_table(table);
            }
        }
    };
}


pub enum Action<I, IT: Iterator<Item=I>> {
    /// Nothing more needs to be iterated over.
    Stop,
    /// Get the next row.
    Next,
    /// Remove the row that was just passed in.
    Remove,
    /// Add an arbitrary number of rows, after the provided row, using a move iterator.
    /// The rows inserted in this manner will not be walked by the closure.
    /// If you want to do a Remove and Add at the same time, move the first item in the iterator
    /// into the passed in row.
    Add(IT),
}


fn check_name(name: &str) {
    match name.chars().next() {
        None => panic!("Empty name"),
        Some('_') => panic!("Reserved name {}", name),
        Some(c) if (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') => (),
        _ => panic!("Invalid name {}", name),
    }
    for c in name.chars() {
        if c == '_' { continue; }
        if c >= 'A' && c <= 'Z' { continue; }
        if c >= 'a' && c <= 'z' { continue; }
        if c >= '0' && c <= '9' { continue; }
        panic!("Invalid name {}", name);
    }
}

/* Still need to get a JOIN solution! */



#[cfg(test)]
#[allow(dead_code)]
mod tests {
    mod table_use {
        #[derive(Clone, Copy, PartialEq, PartialOrd, Debug, RustcEncodable, RustcDecodable)]
        pub enum CheeseKind {
            Swiss,
            Stinky,
            Brie,
        }
        impl Default for CheeseKind {
            fn default() -> Self { CheeseKind::Stinky }
        }
        impl ::Storable for CheeseKind { }
    }
    use self::table_use::*;
    
    table! {
        [cheese],
        mass: usize,
        holes: u16,
        kind: CheeseKind,
    }


    #[test]
    #[should_panic(expected = "Invalid name 123")]
    fn bad_name() {
        let mut universe = ::Universe::new();
        cheese::with_name("123").register(&mut universe);
    }

    #[test]
    fn table_test() {
        let mut universe = ::Universe::new();
        cheese::default().register(&mut universe);

        {
            let mut cheese = cheese::default().write(&universe);
            cheese.push(cheese::Row {
                mass: 1000usize,
                holes: 20,
                kind: CheeseKind::Stinky,
            });
        }
    }
}

