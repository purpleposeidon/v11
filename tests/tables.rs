#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;

extern crate rustc_serialize;


domain! { TEST }
use v11::{Universe, Action};

table! {
    pub [TEST/new_table_test] {
        random_number: [usize; VecCol<usize>],
    }
    impl {
        RowId = u8;
    }
    mod {
        fn hello() {
            println!("Hey!");
        }
    }
}

table! {
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
    [TEST/cheese] {
        mass: [usize; VecCol<usize>],
        holes: [u16; VecCol<u16>],
        kind: [CheeseKind; VecCol<CheeseKind>],
    }
    mod {
        use super::CheeseKind;
    }
}

table! {
    [TEST/test_foreign] {
        id: [EasyRowId; VecCol<EasyRowId>],
    }
    mod {
        use super::EasyRowId;
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
        sortie::register();
        bsortie::register();
        bits::register();
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
    }
    let cheese = cheese::read(&universe);
    for i in cheese.range() {
        println!("{:?}", cheese.get_row(i));
    }
}
fn dump(easy: &mut easy::Write) {
    for i in easy.range() {
        println!("{:?}", easy.get_row(i));
    }
}

#[test]
fn visit_remove() {
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    easy.push(easy::Row {x: 1});
    dump(&mut easy);
    for d in 2..10 {
        let mut first = true;
        easy.visit(|easy, i| {
            if d == 2 && !first {
                panic!("visiting stuff I just made! {:?} {:?}", easy.get_row(i), i);
            }
            first = false;
            Action::Add(Some(easy::Row { x: easy.x[i] * d }).into_iter())
        });
        println!("d = {}", d);
        dump(&mut easy);
    }
    easy.visit(|easy, i| -> easy::ClearVisit {
        if easy.x[i] % 10 == 0 {
            Action::Remove
        } else {
            Action::Continue
        }
    });
    println!("Some 0's removed:");
    dump(&mut easy);
}

#[test]
fn visit_break_immediate() {
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    easy.push(easy::Row {x: 1});
    easy.visit(|_, _| -> easy::ClearVisit { Action::Break } );
}

#[test]
fn visit_add() {
    fn dump(easy: &mut easy::Write) {
        for i in easy.range() {
            println!("{:?}", easy.get_row(i));
        }
    }
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    easy.push(easy::Row {x: 1});
    //dump(&mut easy);
    for d in 2..10 {
        let mut first = true;
        easy.visit(|easy, i| {
            if d == 2 && !first {
                panic!("visiting stuff I just made! {:?} {:?}", easy.get_row(i), i);
            }
            first = false;
            Action::Add(Some(easy::Row { x: easy.x[i] * d }).into_iter())
        });
        //println!("d = {}", d);
        //dump(&mut easy);
    }
}

// These two aren't very good tests. Just don't panic, I guess.
#[test]
fn visit_remove_break() {
    fn b() -> easy::ClearVisit { Action::Break }
    visit_remove_and(b);
}

#[test]
fn visit_remove_continue() {
    fn c() -> easy::ClearVisit { Action::Continue }
    visit_remove_and(c);
}

fn visit_remove_and<A: Fn() -> easy::ClearVisit>(act: A) {
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    for n in 0..10 {
        easy.push(easy::Row {x: n});
    }
    dump(&mut easy);
    let mut n = 0;
    easy.visit(|_, _| -> easy::ClearVisit {
        n += 1;
        if n > 5 {
            act()
        } else {
            Action::Remove
        }
    });
    println!("After stuff was removed:");
    dump(&mut easy);
}

#[test]
fn remove_one() {
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    for i in 0..2 {
        easy.push(easy::Row { x: i });
    }
    let mut first = true;
    println!("Start");
    dump(&mut easy);
    assert_eq!(easy.rows(), 2);
    easy.visit(|_, _| -> easy::ClearVisit {
        if first {
            first = false;
            Action::Remove
        } else {
            Action::Break
        }
    });
    println!("");
    dump(&mut easy);
    assert_eq!(easy.rows(), 1);
}

table! {
    [TEST/sortie] {
        i: [usize; VecCol<usize>],
    }
    impl {
        RowId = usize;
        SortBy(i);
    }
}

#[test]
fn sort() {
    let universe = make_universe();
    println!("Input:");
    let orig_len = {
        let mut sortie = sortie::write(&universe);
        for i in 0..40 {
            let i = 40 - i;
            println!("{}", i);
            sortie.push(sortie::Row { i: i });
        }
        sortie.rows()
    };
    let mut sortie = sortie::write(&universe);
    sortie.sort_by_i();
    println!("Sorted:");
    for i in sortie.range() {
        println!("{}", sortie.i[i]);
    }
    assert_eq!(orig_len, sortie.rows());
}



table! {
    [TEST/bsortie] {
        i: [bool; BoolCol],
    }
    impl {
        SortBy(i);
    }
}

#[test]
fn bsort() {
    let universe = make_universe();
    let orig_len = {
        let mut bsortie = bsortie::write(&universe);
        bsortie.push(bsortie::Row { i: false });
        bsortie.push(bsortie::Row { i: false });
        bsortie.push(bsortie::Row { i: true });
        bsortie.push(bsortie::Row { i: false });
        bsortie.push(bsortie::Row { i: true });
        bsortie.rows()
    };
    let mut bsortie = bsortie::write(&universe);
    bsortie.sort_by_i();
    println!("Sorted:");
    for i in bsortie.range() {
        println!("{:?}", bsortie.get_row(i));
    }
    assert_eq!(orig_len, bsortie.rows());
    assert_eq!(bsortie.dump().iter().map(|r| { r.i }).collect::<Vec<_>>(), &[false, false, false, true, true]);
}

table! {
    [TEST/bits] {
        a: [bool; BoolCol],
        b: [bool; VecCol<bool>],
    }
    impl {
        SortBy(a);
        SortBy(b);
    }
}


#[test]
fn bool_col() {
    let universe = make_universe();
    {
        let mut bits = bits::write(&universe);
        bits.push(bits::Row { a: true, b: true });
        bits.push(bits::Row { a: false, b: false });
        bits.push(bits::Row { a: true, b: true });
        bits.push(bits::Row { a: false, b: false });
        println!("{}", bits.rows());
    }
    {
        {
            let mut bits = bits::write(&universe);
            bits.sort_by_a();
            println!("{}", bits.rows());
        }
        {
            let mut bits = bits::write(&universe);
            bits.sort_by_b();
            println!("{}", bits.rows());
        }
        let mut bits = bits::write(&universe);
        bits.sort_by_a();
        println!("");
        println!("");
        for i in bits.range() {
            println!("{:?}", i);
        }
        for i in bits.range() {
            println!("{:?}", i);
            println!("{:?}", bits.get_row(i));
        }
    }
}


#[test]
fn push() {
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    let er = easy.push(easy::Row { x: 1 });
    assert_eq!(er.to_usize(), 0);
}

#[test]
fn compile_rowid_in_hashmap() {
    #![allow(unused_variables)]
    use std::collections::HashMap;
    let x: HashMap<easy::RowId, ()> = HashMap::new();
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    let er = easy.push(easy::Row { x: 1 });
}

table! {
    [TEST/test_u16] {
        x: [i32; VecCol<i32>],
    }
    impl {
        RowId = u16;
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
}

#[test]
fn contains() {
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    assert!(!easy.contains(easy::at(1)));
    let a = easy.push(easy::Row {x: 1});
    assert!(easy.contains(a));
    assert!(!easy.contains(easy::at(2)));
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
    pub [TEST/compile_serialization] {
        random_number: [usize; VecCol<usize>],
    }
    impl {
        Save;
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
