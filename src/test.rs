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
    [cheese],
    mass: usize,
    holes: u16,
    kind: CheeseKind,
}

table! {
    [easy],
    x: i32,
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
fn visit_break() {
    let mut universe = ::Universe::new();
    easy::default().register(&mut universe);
    let mut easy = easy::write(&universe);
    for n in 0..10 {
        easy.push(easy::Row {x: n});
    }
    dump(&mut easy);
    easy.visit(|_, i| -> easy::ClearVisit {
        if unsafe { i.extricate() } > 5 {
            ::Action::Break
        } else {
            ::Action::Remove
        }
    });
    println!("Stuff removed!");
    dump(&mut easy);
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
