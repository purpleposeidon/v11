#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;





// FIXME: Test sorting.

// FIXME: Can this go into "tests/" as is standard? Previous usage of build.rs probably made that
// difficult.

domain! { pub DOCTEST }
table! {
    /// Can we document the table?
    pub [DOCTEST/documentation] {
        /// Can we document this column?
        /// FIXME: The answer is no.
        documented_column: [bool; BoolCol],
        undocumented_column: [bool; BoolCol],
    }
}
