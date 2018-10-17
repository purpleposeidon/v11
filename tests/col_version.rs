#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;


domain! { TEST }
use v11::Universe;

table! {
    #[kind = "append"]
    #[row_id = "u8"]
    pub [TEST/new_table_test] {
        random_number: [usize; VecCol<usize>],
        #[version = "1"]
        silly_number: [usize; VecCol<usize>],
    }
}



fn make_universe() -> Universe {
    // Prevent lock clobbering breaking tests w/ threading.
    use std::sync::{Once, ONCE_INIT};
    static REGISTER: Once = ONCE_INIT;
    REGISTER.call_once(|| {
        TEST.register();
        new_table_test::register();
    });
    Universe::new(&[TEST])
}

#[test]
fn main() {
    make_universe();
}
