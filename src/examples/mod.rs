domain! { EXAMPLE_DOMAIN }

table! {
    #[kind="append"]
    pub [EXAMPLE_DOMAIN/example_table] {
        foo: [i32; VecCol<i32>],
        bits: [bool; BoolCol],
    }
}

property! { pub static EXAMPLE_DOMAIN/MY_PROPERTY: u32 = 42; }

context! {
    pub struct MyContext {
        pub example_table: self::example_table::Write,
        pub my_property: self::MY_PROPERTY::Read,
    }
}
