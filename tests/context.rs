#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;

use v11::Universe;


domain! { pub TESTS }

table! {
    [TESTS/cheeses] {
        color: [u32; SegCol<u32>],
    }
}

table! {
    [TESTS/stenches] {
        stinkiness: [f32; SegCol<f32>],
    }
}

table! {
    [TESTS/wines] {
        alcohols: [u64; SegCol<u64>],
    }
}

context! {
    mod cheese_mod;
    pub struct CheeseCtx {
        stinkiness: stenches::Write,
    }
}
context! {
    mod full_mod;
    pub struct FullCtx {
        cheeses: cheeses::Read,
        stinkiness: stenches::Write,
        alcohols: wines::Read,
    }
}

context! {
    mod reduced_mod;
    pub struct ReducedCtx {
        cheeses: cheeses::Read,
    }
}

property! { pub static TESTS/SUMPROP: usize = 10; }

context! {
    mod with_props;
    pub struct WithPropsCtx {
        sumprop: SUMPROP::Write,
        cheeses: cheeses::Read,
    }
}


fn new_verse() -> Universe {
    TESTS.register();
    cheeses::register();
    stenches::register();
    wines::register();
    SUMPROP.register();
    Universe::new(&[TESTS])
}

#[test]
fn main() {
    let universe = &new_verse();
    let mut cheese = CheeseCtx::new(universe);
    cheese.stinkiness.push(stenches::Row {
        stinkiness: 237.0,
    });
    let mut full = FullCtx::from(universe, cheese);
    full.stinkiness.push(stenches::Row {
        stinkiness: 238.0,
    });
    let reduced = ReducedCtx::from(universe, full);
    for row in reduced.cheeses.iter() {
        panic!("Well that's odd. {:?}", row);
    }

    let mut wprops = WithPropsCtx::from(universe, reduced);
    *wprops.sumprop += 10;
    assert_eq!(*wprops.sumprop, 20);
}
