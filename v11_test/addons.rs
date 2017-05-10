
use tables::TEST;

table! {
    [TEST/yes_debug] {
        foo: [i32; SegCol<i32>],
    }
    impl {
        // ...
    }
}
table! {
    [TEST/no_debug] {
        foo: [i32; SegCol<i32>],
    }
    impl {
        NoDebug;
    }
}


use v11::Universe;

#[test]
fn compiles_with_or_without_debug() {
    TEST.register();
    TEST.set_locked(false); // gotta cheat for tests!
    yes_debug::register();
    no_debug::register();
    Universe::new(&[]);
}
