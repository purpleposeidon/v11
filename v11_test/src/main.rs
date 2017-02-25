#[macro_use]
extern crate v11;

new_table! {
    pub new_table_test {
        random_number: [usize; VecCol<usize>],
    }
    impl {
        RowId = u8;
    }
    mod {
        fn hello() {
            println!("Hey!");
        }
    }
}

fn main() {
    println!("Hello, world!");
}
