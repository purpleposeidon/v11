#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;
extern crate rustc_serialize;

#[derive(Debug, PartialEq)]
pub struct NotCopy;

domain! { TEST }

table! {
    [TEST/notcopy] {
        foo: [NotCopy; VecCol<NotCopy>],
    }
    impl {
        NoCopy;
        NoClone;
    }
}


