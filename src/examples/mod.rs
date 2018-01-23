domain! { EXAMPLE_DOMAIN }

table! {
    #[kind="append"]
    pub [EXAMPLE_DOMAIN/example_table] {
        foo: [i32; VecCol<i32>],
        bits: [bool; BoolCol],
    }
}
