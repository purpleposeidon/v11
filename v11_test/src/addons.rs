
table! {
    yes_debug {
        foo: [i32; SegCol<i32>],
    }
    impl {
        // ...
    }
}
table! {
    no_debug {
        foo: [i32; SegCol<i32>],
    }
    impl {
        NoDebug;
    }
}
