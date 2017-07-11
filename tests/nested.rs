

#[macro_use]
extern crate v11;
use v11::*;

domain! { TEST }

property! { static TEST/NEST: Option<Universe> }

#[test]
fn test() {
    TEST.register();
    NEST.register();
    let inner = Universe::new(&[]);
    let outer = Universe::new(&[TEST]);
    *outer[NEST].write().unwrap() = Some(inner);
}
