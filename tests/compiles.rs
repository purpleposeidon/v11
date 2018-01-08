#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;

extern crate rustc_serialize;


domain! { TEST }


table! {
    [TEST/hello_there] {
        foo: [i32; SegCol<i32>],
    }
    impl {
        // FIXME: This should crash.
        // ...
    }
}
