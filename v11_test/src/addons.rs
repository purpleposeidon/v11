
use tables::TEST;

table! {
    TEST/yes_debug {
        foo: [i32; SegCol<i32>],
    }
    impl {
        // ...
    }
}
table! {
    TEST/no_debug {
        foo: [i32; SegCol<i32>],
    }
    impl {
        NoDebug;
    }
}


use v11::Universe;

#[test]
fn compiles_with_or_without_debug() {
    TEST.register_domain();
    yes_debug::register();
    no_debug::register();
    Universe::new(&[]);
}
