//! Example generated code.
//!
//! (This module is only present at doc-time.)

// FIXME: None of the doc-strings show up. :|



/// This is an example of how to create a domain.
domain! { pub EXAMPLE_DOMAIN }



/// Ahh, this documentation goes on the module...
table! {
    /// This is an append-only table.
    #[kind="append"]
    pub [EXAMPLE_DOMAIN/example_table] {
        foo: [i32; VecCol<i32>],
        bits: [bool; BoolCol],
    }
}

/// This is a property initialized by an expression.
property! { pub static EXAMPLE_DOMAIN/MY_PROPERTY: u32 = 42; }

/// This is a context.
context! {
    pub struct ExampleContext {
        pub example_table: self::example_table::Write,
        pub my_property: self::MY_PROPERTY::Read,
    }
}
