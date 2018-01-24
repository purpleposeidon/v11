#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;

use v11::Universe;

domain! { TEST }

fn make_universe() -> Universe {
    // Prevent lock clobbering breaking tests w/ threading.
    use std::sync::{Once, ONCE_INIT};
    static REGISTER: Once = ONCE_INIT;
    REGISTER.call_once(|| {
        TEST.register();
        sorted::register();
    });
    Universe::new(&[TEST])
}

table! {
    #[kind = "sorted"]
    #[row_derive(Debug, Clone)]
    pub [TEST/sorted] {
        #[sort_key]
        key: [u8; VecCol<u8>],
        val: [&'static str; VecCol<&'static str>],
    }
}

#[test]
fn is_sortable() {
    let universe = &make_universe();
    let mut sorted = sorted::write(universe);
    sorted.merge(vec![
        sorted::Row {
            key: 1,
            val: "alice",
        },
        sorted::Row {
            key: 5,
            val: "bob",
        },
        sorted::Row {
            key: 2,
            val: "charles",
        },
        sorted::Row {
            key: 33,
            val: "eve",
        },
        sorted::Row {
            key: 3,
            val: "denis",
        },
        sorted::Row {
            key: 4,
            val: "elizabeth",
        },
        sorted::Row {
            key: 0,
            val: "aardvarken",
        }
    ]);
    let mut prev = 0;
    for row in sorted.iter() {
        println!("{:?}", sorted.get_row(row));
        assert!(prev <= sorted.key[row]);
        prev = sorted.key[row];
    }
}
