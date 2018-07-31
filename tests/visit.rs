extern crate rand;
#[macro_use] extern crate v11;
#[macro_use] extern crate v11_macros;

use v11::{Universe, Action};

domain! { TEST }

fn make_universe() -> Universe {
    // Prevent lock clobbering breaking tests w/ threading.
    use std::sync::{Once, ONCE_INIT};
    static REGISTER: Once = ONCE_INIT;
    REGISTER.call_once(|| {
        TEST.register();
        easy::register();
    });
    Universe::new(&[TEST])
}


table! {
    #[kind = "append"]
    #[row_derive(Clone, Debug, PartialEq)]
    [TEST/easy] {
        x: [i32; VecCol<i32>],
    }
}



fn dump(easy: &mut easy::Write) {
    for i in easy.iter() {
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
            Action::Continue {
                remove: false,
                add: Some(easy::Row { x: easy.x[i] * d }),
            }
        });
        println!("d = {}", d);
        dump(&mut easy);
    }
    easy.retain(|easy, i| {
        easy.x[i] % 10 != 0
    });
    println!("Some 0's removed:");
    dump(&mut easy);
}

#[test]
fn visit_break_immediate() {
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    easy.push(easy::Row {x: 1});
    easy.visit(|_, _| -> EasyAction { Action::Break });
}

#[test]
fn visit_add() {
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
            Action::Continue {
                remove: false,
                add: Some(easy::Row { x: easy.x[i] * d }).into_iter(),
            }
        });
        //println!("d = {}", d);
        //dump(&mut easy);
    }
}

type EasyAction = Action<Option<easy::Row>>;

// These two aren't very good tests. Just don't panic, I guess.
#[test]
fn visit_remove_break() {
    fn b() -> EasyAction { Action::Break }
    visit_remove_and(b);
}

#[test]
fn visit_remove_continue() {
    fn c() -> EasyAction {
        Action::Continue {
            remove: false,
            add: None,
        }
    }
    visit_remove_and(c);
}

fn visit_remove_and<A: Fn() -> EasyAction>(act: A) {
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    for n in 0..10 {
        easy.push(easy::Row {x: n});
    }
    dump(&mut easy);
    let mut n = 0;
    easy.visit(|_, _| -> EasyAction {
        n += 1;
        if n > 5 {
            act()
        } else {
            Action::Continue {
                remove: true,
                add: None,
            }
        }
    });
    println!("After stuff was removed:");
    dump(&mut easy);
}

#[test]
fn do_nothing() {
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    easy.visit(|_, _| -> EasyAction {
        panic!();
    });
}

#[test]
fn double() {
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    for x in 0..3 {
        easy.push(easy::Row { x });
    }
    easy.visit(|easy, rowid| -> EasyAction {
        Action::Continue {
            remove: false,
            add: Some(easy::Row { x: easy.x[rowid] * 2 }),
        }
    });
    for row in easy.iter() {
        println!("{:?}", easy.get_row(row));
    }
    println!("...");
    for i in 0..3 {
        let row = easy::RowId::from_usize(i * 2);
        println!("{:?} = {:?}\nnext =    {:?}", row, easy.get_row(row), easy.get_row(row.next()));
        assert_eq!(easy.x[row] * 2, easy.x[row.next()]);
    }
}

#[test]
fn remove_all() {
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    for x in 0..5 {
        easy.push(easy::Row { x });
    }
    easy.visit(|_, _| -> EasyAction {
        Action::Continue {
            remove: true,
            add: None,
        }
    });
    assert_eq!(easy.len(), 0);
}

#[test]
fn weave() {
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    for _ in 0..4 {
        easy.push(easy::Row { x: 1 });
    }
    easy.visit(|_, _| -> EasyAction {
        Action::Continue {
            remove: false,
            add: Some(easy::Row { x: 0 }),
        }
    });
    let mut expect = 1;
    for row in easy.iter() {
        println!("{:?}", easy.get_row(row));
        //assert_eq!(easy.x[row], expect);
        expect = if expect == 1 { 0 } else { 1 };
    }
}

#[test]
fn empty_then_flood() {
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    for x in 0..4 {
        easy.push(easy::Row { x });
    }
    easy.visit(|_easy, i| {
        Action::Continue {
            remove: true,
            add: if i.to_usize() == 3 {
                (0..10).map(|x| easy::Row { x }).collect()
            } else {
                vec![]
            },
        }
    });
    for row in easy.iter() {
        println!("{:?}", row);
        assert_eq!(row.to_usize() as i32, easy.x[row]);
    }
}

#[test]
fn remove_one_way() {
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    for i in 2..4 {
        easy.push(easy::Row { x: i });
    }
    println!("Start");
    dump(&mut easy);
    assert_eq!(easy.len(), 2);
    let mut first = true;
    easy.visit(|_, _| -> EasyAction {
        if first {
            first = false;
            Action::Continue {
                remove: true,
                add: None,
            }
        } else {
            Action::Break
        }
    });
    println!("");
    dump(&mut easy);
    assert_eq!(easy.len(), 1);
}

#[test]
fn remove_other_way() {
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    for i in 2..4 {
        easy.push(easy::Row { x: i });
    }
    println!("Start");
    dump(&mut easy);
    assert_eq!(easy.len(), 2);
    let mut first = true;
    easy.visit(|_, _| -> EasyAction {
        let remove = !first;
        first = false;
        Action::Continue {
            remove,
            add: None,
        }
    });
    println!("");
    dump(&mut easy);
    assert_eq!(easy.len(), 1);
}



#[test]
fn push() {
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    let er = easy.push(easy::Row { x: 1 });
    assert_eq!(er.to_usize(), 0);
}

use rand::{Rng, SeedableRng, XorShiftRng};

#[test]
fn random() {
    let mut rng = XorShiftRng::from_seed(Default::default());
    for _ in 0..10000 {
        random_round(rng.gen());
    }
}

fn random_round(seed: [u8; 16]) {
    let mut rng = XorShiftRng::from_seed(seed);
    let check: Vec<easy::Row> = (0..rng.gen_range(0, 20))
        .map(|_| {
            easy::Row { x: rng.gen() }
        }).collect();
    type CheckAction = Action<Vec<easy::Row>>;
    let actions: Vec<CheckAction> = check
        .iter()
        .map(|_| {
            Action::Continue {
                remove: rng.gen(),
                add: if rng.gen() {
                    (0..rng.gen_range(1, 3))
                        .map(|_| {
                            easy::Row { x: rng.gen() }
                        }).collect()
                } else {
                    vec![]
                },
            }
        }).collect();
    let universe = make_universe();
    let mut easy = easy::write(&universe);
    for row in &check {
        easy.push(row.clone());
    }

    let mut checked = Vec::new();
    for (row, action) in check.iter().zip(actions.iter()) {
        match action {
            &Action::Continue { remove, ref add } => {
                if !remove {
                    checked.push(row.clone());
                }
                for row in add {
                    checked.push(row.clone());
                }
            },
            &Action::Break => panic!(),
        }
    }

    let mut actions = actions.iter();
    easy.visit(move |_, _| {
        actions.next().expect("ran out of actions!").clone()
    });

    assert_eq!(easy.len(), checked.len());

    //println!("Did some random stuff:");
    for (a, b) in easy.dump().iter().zip(checked.iter()) {
        assert_eq!(a, b);
        //println!("{:?} == {:?}", a, b);
    }
}
