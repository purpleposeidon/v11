#[macro_use] extern crate v11_macros;
#[macro_use] extern crate v11;

use v11::Universe;

domain! { TEST }

table! {
    #[kind = "sorted"]
    #[row_derive(Clone, Debug)]
    pub [TEST/table] {
        #[sort_key]
        val: [usize; VecCol<usize>],
    }
}

fn make_universe() -> Universe {
    // Prevent lock clobbering breaking tests w/ threading.
    use std::sync::{Once, ONCE_INIT};
    static REGISTER: Once = ONCE_INIT;
    REGISTER.call_once(|| {
        TEST.register();
        table::register();
    });
    Universe::new(&[TEST])
}

fn pass(vals: &[usize], rm: &[usize], expect: &[usize]) {
    let universe = &make_universe();
    {
        let mut table = table::write(universe);
        table.merge(vals.iter().map(|i| table::Row { val: *i }).collect::<Vec<_>>());
        table.flush(universe, ::v11::event::CREATE);
    }
    {
        println!();
        println!("Start:");
        let table = table::read(universe);
        for r in table.iter() {
            let m = if rm.contains(&r.to_usize()) { '*' } else { ' ' };
            println!("{}{}\t{:?}", r.to_usize(), m, table.get_row(r));
        }
        println!("remove: {:?}", rm);
    }
    {
        let mut table = table::write(universe);
        let r = rm.iter().map(|i| table::RowId::from_usize(*i)).collect::<Vec<_>>();
        let r = r.as_slice();
        table.remove_rows(r);
        table.flush(universe, ::v11::event::CREATE);
    }
    {
        println!();
        println!("Becomes:");
        let table = table::read(universe);
        for r in table.iter() {
            println!("{}\t{:?}", r.to_usize(), table.get_row(r));
        }
        println!();
    }
    {
        let table = table::read(universe);
        let mut n = 0;
        for _ in table.iter() {
            n += 1;
        }
        assert_eq!(n, vals.len() - rm.len());
    }
    {
        let table = table::read(universe);
        for i in 0..expect.len() {
            assert_eq!(
                expect[i],
                table.val[table::RowId::from_usize(i)],
            );
        }
    }
}

#[test]
fn empty() {
    pass(&[], &[], &[])
}

#[test]
fn basic() {
    pass(
        &[0, 1],
        &[   1],
        &[0   ],
    );
}

#[test]
fn medium() {
    pass(
        &[0, 1, 2, 3, 4, 6, 8, 9, 11],
        &[0,       3, 4             ],
        &[   1, 2,       6, 8, 9, 11],
    );
}

#[test]
fn long_and_sparse() {
    pass(
        &[0, 1, 2, 3, 4, 5],
        &[                ],
        &[0, 1, 2, 3, 4, 5],
    );
    pass(
        &[0, 1, 2, 3, 4, 5],
        &[0,             5],
        &[   1, 2, 3, 4   ],
    );
}

#[test]
fn long_and_dense() {
    pass(
        &[0, 1, 2, 3, 4, 5],
        &[0, 1, 2, 3, 4, 5],
        &[                ],
    );
    pass(
        &[0, 1, 2, 3, 4, 5],
        &[   1, 2, 3, 4, 5],
        &[0,              ],
    );
    pass(
        &[0, 1, 2, 3, 4, 5],
        &[0, 1,    3, 4, 5],
        &[      2,        ],
    );
    pass(
        &[0, 1, 2, 3, 4, 5],
        &[0, 1, 2, 3,    5],
        &[            4   ],
    );
    pass(
        &[0, 1, 2, 3, 4, 5],
        &[0, 1, 2, 3, 4   ],
        &[               5],
    );
}

#[test]
#[should_panic(expected = "dupes")]
fn dupe_index() {
    pass(
        &[0, 1, 2, 3],
        &[0, 0],
        &[96],
    );
}

#[test]
#[should_panic(expected = "sorted")]
fn unsorted() {
    pass(
        &[0, 1, 2, 3],
        &[3, 2],
        &[96],
    );
}

