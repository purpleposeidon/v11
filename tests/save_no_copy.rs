#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;
#[macro_use]
extern crate serde_derive;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[derive(Serialize, Deserialize)]
pub struct NotCopy;

domain! { TEST }

table! {
    #[kind = "consistent"]
    #[row_derive(Clone)]
    #[save]
    [TEST/save_no_copy] {
        index: [usize; VecCol<usize>],
        foo: [NotCopy; VecCol<NotCopy>],
    }
}
