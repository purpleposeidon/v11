#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NotCopy;

domain! { TEST }

table! {
    #[kind = "append"]
    [TEST/notcopy] {
        index: [usize; VecCol<usize>],
        foo: [NotCopy; VecCol<NotCopy>],
    }
}
