#![allow(dead_code)]


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
    [pub cheese],
    mass: [usize; VecCol<usize>],
    holes: [u16; VecCol<u16>],
    kind: [CheeseKind; VecCol<CheeseKind>],
}

table! {
    [pub easy],
    x: [i32; VecCol<i32>],
}


#[test]
#[should_panic(expected = "Invalid name 123")]
fn bad_name() {
    let mut universe = ::Universe::new();
    cheese::with_name("123").register(&mut universe);
}

#[test]
fn small_table() {
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

#[test]
fn large_table() {
    let mut universe = ::Universe::new();
    cheese::default().register(&mut universe);
    let mut cheese = cheese::default().write(&universe);
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
    let mut universe = ::Universe::new();
    cheese::default().register(&mut universe);
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
        println!("{:?}", cheese.row(i));
    }
}
fn dump(easy: &mut easy::Write) {
    for i in easy.range() {
        println!("{:?}", easy.row(i));
    }
}

#[test]
fn visit_remove() {
    let mut universe = ::Universe::new();
    easy::default().register(&mut universe);
    let mut easy = easy::write(&universe);
    easy.push(easy::Row {x: 1});
    dump(&mut easy);
    for d in 2..10 {
        let mut first = true;
        easy.visit(|easy, i| {
            if d == 2 && !first {
                panic!("visiting stuff I just made! {:?} {:?}", easy.row(i), i);
            }
            first = false;
            ::Action::Add(Some(easy::Row { x: easy.x[i] * d }).into_iter())
        });
        println!("d = {}", d);
        dump(&mut easy);
    }
    easy.visit(|easy, i| -> easy::ClearVisit {
        if easy.x[i] % 10 == 0 {
            ::Action::Remove
        } else {
            ::Action::Continue
        }
    });
    println!("Some 0's removed:");
    dump(&mut easy);
}

#[test]
fn visit_break_immediate() {
    let mut universe = ::Universe::new();
    easy::default().register(&mut universe);
    let mut easy = easy::write(&universe);
    easy.push(easy::Row {x: 1});
    easy.visit(|_, _| -> easy::ClearVisit { ::Action::Break } );
}

#[test]
fn visit_add() {
    fn dump(easy: &mut easy::Write) {
        for i in easy.range() {
            println!("{:?}", easy.row(i));
        }
    }
    let mut universe = ::Universe::new();
    easy::default().register(&mut universe);
    let mut easy = easy::write(&universe);
    easy.push(easy::Row {x: 1});
    //dump(&mut easy);
    for d in 2..10 {
        let mut first = true;
        easy.visit(|easy, i| {
            if d == 2 && !first {
                panic!("visiting stuff I just made! {:?} {:?}", easy.row(i), i);
            }
            first = false;
            ::Action::Add(Some(easy::Row { x: easy.x[i] * d }).into_iter())
        });
        //println!("d = {}", d);
        //dump(&mut easy);
    }
}

// These two aren't very good tests. Just don't panic, I guess.
#[test]
fn visit_remove_break() {
    fn b() -> easy::ClearVisit { ::Action::Break }
    visit_remove_and(b);
}

#[test]
fn visit_remove_continue() {
    fn c() -> easy::ClearVisit { ::Action::Continue }
    visit_remove_and(c);
}

fn visit_remove_and<A: Fn() -> easy::ClearVisit>(act: A) {
    let mut universe = ::Universe::new();
    easy::default().register(&mut universe);
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
            ::Action::Remove
        }
    });
    println!("After stuff was removed:");
    dump(&mut easy);
}

#[test]
fn remove_one() {
    let mut universe = ::Universe::new();
    easy::default().register(&mut universe);
    let mut easy = easy::write(&universe);
    for i in 0..2 {
        easy.push(easy::Row { x: i });
    }
    let mut first = true;
    println!("Start");
    dump(&mut easy);
    assert!(easy.rows() == 2);
    easy.visit(|_, _| -> easy::ClearVisit {
        if first {
            first = false;
            ::Action::Remove
        } else {
            ::Action::Break
        }
    });
    println!("");
    dump(&mut easy);
    assert!(easy.rows() == 1);
}

table! {
    [pub sortie],
    i: [usize; VecCol<usize>],
}

#[test]
fn sort() {
    let mut universe = ::Universe::new();
    sortie::default().register(&mut universe);
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
    let sortie = sortie::sorted(&universe);
    println!("Sorted:");
    for i in sortie.range() {
        println!("{}", sortie.i[i]);
    }
    assert_eq!(orig_len, sortie.rows());
}

table! {
    [pub bsortie],
    i: [bool; BoolCol],
}

#[test]
fn bool_col_unit() {
    let mut bc = ::BoolCol::new();
    let v = &[true, false, true];
    for i in v {
        bc.push(*i);
    }
    println!("");
    println!("Start:");
    for i in bc.data.iter() {
        println!("{}", i);
    }
    println!("Cleared:");
    bc.clear();
    for i in bc.data.iter() {
        println!("{}", i);
    }
    println!("Really Cleared:");
    bc.data.clear();
    for i in bc.data.iter() {
        println!("{}", i);
    }
    println!("Append:");
    bc.append(&mut vec![true, true]);
    for i in bc.data.iter() {
        println!("{}", i);
    }
    println!("{:?}", bc);
}

#[test]
fn bsort() {
    let mut universe = ::Universe::new();
    bsortie::default().register(&mut universe);
    let orig_len = {
        let mut bsortie = bsortie::write(&universe);
        bsortie.push(bsortie::Row { i: false });
        bsortie.push(bsortie::Row { i: false });
        bsortie.push(bsortie::Row { i: true });
        bsortie.push(bsortie::Row { i: false });
        bsortie.push(bsortie::Row { i: true });
        bsortie.rows()
    };
    let bsortie = bsortie::sorted(&universe);
    println!("Sorted:");
    for i in bsortie.range() {
        println!("{:?}", bsortie.row(i));
    }
    assert_eq!(orig_len, bsortie.rows());
    assert_eq!(bsortie.dump().iter().map(|r| { r.i }).collect::<Vec<_>>(), &[false, false, false, true, true]);
}

table! {
    [pub bits],
    a: [bool; BoolCol],
    b: [bool; VecCol<bool>],
}


#[test]
fn bool_col() {
    let mut universe = ::Universe::new();
    bits::default().register(&mut universe);
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
            let bits = bits::sorted(&universe);
            println!("{}", bits.rows());
        }
        {
            let bits = bits::sorted(&universe);
            println!("{}", bits.rows());
        }
        let bits = bits::sorted(&universe);
        println!("");
        println!("");
        for i in bits.range() {
            println!("{:?}", i);
        }
        for i in bits.range() {
            println!("{:?}", i);
            println!("{:?}", bits.row(i));
        }
    }
    //let bits = bits::write(&universe);
}
