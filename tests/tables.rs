#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;


domain! { TEST }
use v11::Universe;
use v11::tracking::prelude::*;

table! {
    #[kind = "append"]
    #[row_id = "u8"]
    pub [TEST/new_table_test] {
        random_number: [usize; VecCol<usize>],
    }
}

table! {
    #[kind = "consistent"]
    #[row_derive(Clone, Debug)]
    [TEST/easy] {
        x: [i32; VecCol<i32>],
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug)]
pub enum CheeseKind {
    Swiss,
    Stinky,
    Brie,
}
impl Default for CheeseKind {
    fn default() -> Self { CheeseKind::Stinky }
}
pub type EasyRowId = easy::RowId;



table! {
    #[kind = "consistent"]
    #[row_derive(Clone, Debug)]
    [TEST/cheese] {
        mass: [usize; VecCol<usize>],
        holes: [u16; VecCol<u16>],
        kind: [CheeseKind; VecCol<CheeseKind>],
    }
}

table! {
    #[kind = "consistent"]
    [TEST/test_foreign] {
        #[foreign]
        #[index]
        id: [EasyRowId; VecCol<EasyRowId>],
    }
}
impl Tracker for test_foreign::track_id_events {
    type Foreign = easy::Row;

    fn sort(&self) -> bool { false }

    fn handle(&self, universe: &Universe, event: Event, rows: SelectRows<Self::Foreign>, function: &dyn event::Function) {
        let mut rows = test_foreign::read(universe).select_id(rows);
        let gt = test_foreign::get_generic_table(universe);
        if function.needs_sort(gt) {
            rows.sort();
        }
        let rows = rows.as_slice();
        let rows = rows.as_any();
        function.handle(universe, gt, event, rows);
    }
}


fn make_universe() -> Universe {
    // Prevent lock clobbering breaking tests w/ threading.
    use std::sync::{Once, ONCE_INIT};
    static REGISTER: Once = ONCE_INIT;
    REGISTER.call_once(|| {
        TEST.register();
        new_table_test::register();
        easy::register();
        cheese::register();
        test_foreign::register();
        test_u16::register();
    });
    Universe::new(&[TEST])
}

#[test]
fn two_universes() {
    make_universe();
    make_universe();
}

#[test]
fn small_table() {
    let universe = make_universe();

    {
        let mut cheese = cheese::write(&universe);
        cheese.push(cheese::Row {
            mass: 1000usize,
            holes: 20,
            kind: CheeseKind::Stinky,
        });
        cheese.flush(&universe, ::v11::event::CREATE);
    }
}

#[test]
fn large_table() {
    let universe = make_universe();
    let mut cheese = cheese::write(&universe);
    for x in 10..1000 {
        cheese.push(cheese::Row {
            mass: x,
            holes: 2000,
            kind: CheeseKind::Swiss,
        });
    }
    cheese.flush(&universe, ::v11::event::CREATE);
}

#[test]
fn walk_table() {
    let universe = make_universe();
    {
        let mut cheese = cheese::write(&universe);
        for x in 0..10 {
            cheese.push(cheese::Row {
                mass: x,
                holes: 2000,
                kind: CheeseKind::Swiss,
            });
        }
        cheese.flush(&universe, ::v11::event::CREATE);
    }
    let cheese = cheese::read(&universe);
    for i in cheese.iter() {
        println!("{:?}", cheese.get_row(i));
    }
}

#[test]
fn compile_rowid_in_hashmap() {
    #![allow(unused_variables)]
    use std::collections::HashMap;
    let x: HashMap<easy::RowId, ()> = HashMap::new();
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    let er = easy.push(easy::Row { x: 1 });
    easy.flush(&universe, event::CREATE);
}

table! {
    #[kind = "consistent"]
    #[row_id = "u16"]
    [TEST/test_u16] {
        x: [i32; VecCol<i32>],
    }
}

#[test]
fn compile_rowid_cmp() {
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    let a = easy.push(easy::Row {x: 1});
    assert!(a == a);
    assert!(a >= a);
    assert!(a >= a);
    let b = easy.push(easy::Row {x: 1});
    assert!(a != b);
    assert!(b > a);
    easy.flush(&universe, event::CREATE);
}

#[test]
fn contains() {
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    assert!(!easy.contains(easy::at(1)));
    let a = easy.push(easy::Row {x: 1});
    assert!(easy.contains(a));
    assert!(!easy.contains(easy::at(2)));
    easy.flush(&universe, event::CREATE);
}


//mod a {
//    mod table_use {}
//    table! {
//        [pub twin],
//        value: [i32; VecCol<i32>],
//    }
//}
//
//mod b {
//    mod table_use {}
//    table! {
//        [pub twin],
//        value: [i32; VecCol<i32>],
//    }
//}
//
//#[test]
//fn foreign_compat() {
//    let mut universe = Universe::new();
//    a::twin::register(&mut universe);
//    {
//        // so 'b::twin' should effectively already be registered?
//        b::twin::read(&universe);
//    }
//
//    b::twin::register(&mut universe);
//    let spot = a::twin::write(&universe).push(a::twin::Row {
//        value: 237,
//    });
//    // Well, we can't use spot...
//    // Like there's probably *nothing whatsoever* we could do, unless we can use strings as type
//    // parameters.
//    let spot = b::twin::at(spot.to_raw());
//    assert_eq!(237, b::twin::read(&universe).value[spot]);
//}

table! {
    #[kind = "append"]
    #[row_derive(Clone)]
    pub [TEST/compile_serialization] {
        random_number: [usize; VecCol<usize>],
    }
}

#[test]
fn lifetimes_are_sane() {
    use std::sync::Arc;
    let universe = Arc::new(make_universe());
    let universe = &universe;
    let first = {
        new_table_test::write(universe).push(new_table_test::Row {
            random_number: 42,
        })
    };
    use std::thread::spawn;
    fn sleep(ms: u64) {
        let d = ::std::time::Duration::from_millis(ms * 10);
        ::std::thread::sleep(d);
    }
    let a = {
        let universe = Arc::clone(universe);
        spawn(move || {
            let universe = &*universe;
            let ohno = {
                &new_table_test::read(universe).random_number[first]
            };
            sleep(2);
            assert_eq!(*ohno, 42);
        })
    };
    let b = {
        let universe = Arc::clone(universe);
        spawn(move || {
            let universe = &*universe;
            sleep(1);
            new_table_test::write(universe).random_number[first] = 0xBAD;
        })
    };
    a.join().unwrap();
    b.join().unwrap();
}

// FIXME: Get a compile-fail test thingie going
/*
#[test]
fn table_locks_are_semisound() {
    loop {
        let universe = &make_universe();
        let first = {
            new_table_test::write(universe).push(new_table_test::Row {
                random_number: 42,
            })
        };
        extern crate rand;
        let rng: bool = ::rand::random();
        let okay = 10;
        let ohno = if rng {
            &new_table_test::read(universe).random_number[first]
        } else {
            println!("bad luck");
            &okay
        };
        if *ohno == 10 {
            continue;
        }
        {
            new_table_test::write(universe).random_number[first] = 0xBAD;
        }
        panic!("Didn't hang, value is: {}", *ohno);
    }
}
*/


// FIXME: Make this test fail to compile
#[test]
fn table_columns_are_unswappable() {
    let u1 = &make_universe();
    let u2 = &make_universe();
    let mut t1 = new_table_test::write(u1);
    let mut t2 = new_table_test::write(u2);
    use std::mem;
    mem::swap(&mut t1.random_number, &mut t2.random_number);
    mem::drop(t2);
    println!("whelp...");
    mem::drop(t1);
}
