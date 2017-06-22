#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;

extern crate rustc_serialize;


domain! { TEST }
use v11::{Universe, Action};



table! {
    pub [TEST/track] {
        number: [i32; VecCol<i32>],
    }
    impl {
        Track;
    }
}

