#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;

use v11::Universe;


domain! { pub TESTS }

table! {
    #[kind = "consistent"]
    [TESTS/cheeses] {
        color: [u32; SegCol<u32>],
    }
}

table! {
    #[kind = "consistent"]
    [TESTS/stenches] {
        stinkiness: [f32; SegCol<f32>],
    }
}

table! {
    #[kind = "consistent"]
    [TESTS/wines] {
        alcohols: [u64; SegCol<u64>],
    }
}

mod cheese_nonce {
    context! {
        pub struct CheeseCtx {
            pub stinkiness: stenches::Write,
        }
    }
}
use self::cheese_nonce::*;
mod full_nonce {
    context! {
        pub struct FullCtx {
            pub cheeses: cheeses::Read,
            pub stinkiness: stenches::Write,
            pub alcohols: wines::Read,
        }
    }
}
use self::full_nonce::*;

mod reduced_nonce {
    context! {
        pub struct ReducedCtx {
            pub cheeses: cheeses::Read,
        }
    }
}
use self::reduced_nonce::*;

property! { pub static TESTS/SUMPROP: usize = 10; }

context! {
    pub struct WithPropsCtx {
        pub sumprop: SUMPROP::Write,
        pub cheeses: cheeses::Read,
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
