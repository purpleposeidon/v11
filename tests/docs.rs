#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;





// FIXME: Test sorting.

domain! { pub DOCTEST }
table! {
    /// Can we document the table? (FIXME: No, we can't.)
    #[kind = "consistent"]
    pub [DOCTEST/documentation] {
        /// Can we document this column?
        documented_column: [bool; BoolCol],
        undocumented_column: [bool; BoolCol],
    }
}

#[test]
fn v11_macro_dump() {}
