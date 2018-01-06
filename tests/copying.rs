#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;
extern crate rustc_serialize;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NotCopy;

domain! { TEST }

table! {
    [TEST/notcopy] {
        index: [usize; VecCol<usize>],
        foo: [NotCopy; VecCol<NotCopy>],
    }
    impl {
        NoCopy;
        NoClone;
        //SortBy(foo);
    }
}
