#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;


domain! { TEST }


table! {
    #[kind = "consistent"]
    #[version = "42"]
    [TEST/hello_there] {
        foo: [i32; SegCol<i32>],
    }
}

#[test]
fn compiles() {
    assert_eq!(hello_there::VERSION, 42);
}
