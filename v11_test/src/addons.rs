
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
    let mut universe = Universe::new(&[]);
    yes_debug::register(&mut universe);
    no_debug::register(&mut universe);
}
