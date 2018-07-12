#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;
extern crate rand;

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

#[test]
fn test_merge_thoroughly() {
    let universe = &make_universe();
    use rand::*;
    for seed in 2..20 {
        let mut rng = XorShiftRng::from_seed([2, 9, 293, seed]);
        let mut sorted = sorted::write(universe);
        for n in 0..rng.gen_range(1, 20) {
            println!("round {}", n);
            let mut new = Vec::new();
            for _ in 0..rng.gen_range(1, 20) {
                new.push(sorted::Row {
                    key: rng.gen(),
                    val: "meh",
                });
            }
            new.sort();
            println!("{:?}", new);
            sorted.merge(new);
            sorted.assert_sorted();
        }
        sorted.clear();
    }
}

#[test]
fn merge_singles() {
    let universe = &make_universe();
    let mut sorted = sorted::write(universe);
    sorted.assert_sorted();
    sorted.merge_in_a_single_row(sorted::Row {
        key: 10,
        val: "ok",
    });
    sorted.assert_sorted();
    sorted.merge_in_a_single_row(sorted::Row {
        key: 5,
        val: "ok",
    });
    sorted.assert_sorted();
    sorted.merge_in_a_single_row(sorted::Row {
        key: 7,
        val: "ok",
    });
    sorted.assert_sorted();
    sorted.close();

    for seed in 3..20 {
        use rand::*;
        let mut rng = XorShiftRng::from_seed([3, 10, 294, seed]);
        let mut sorted = sorted::write(universe);
        for n in 0..30 {
            println!("round {}", n);
            let mut new = Vec::new();
            new.push(sorted::Row {
                key: rng.gen(),
                val: "meh",
            });
            new.sort();
            println!("{:?}", new);
            sorted.merge(new);
            sorted.assert_sorted();
        }
        sorted.clear();
    }
}


impl<'u> sorted::Write<'u> {
    pub fn assert_sorted(&self) {
        for i in self.iter() {
            println!("{}", self.key[i]);
        }
        if self.len() < 2 { return; }
        let mut prev = self.key[sorted::FIRST];
        for i in self.iter().skip(1) {
            let here = self.key[i];
            assert!(prev <= here);
            prev = here;
        }
    }
}
