#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;

extern crate rustc_serialize;


domain! { TEST }
use v11::Universe;
use v11::tracking::Tracker;


table! {
    #[kind = "public"]
    [TEST/track] {
        number: [i32; VecCol<i32>],
    }
}

table! {
    #[kind = "public"]
    [TEST/follow] {
        #[foreign]
        #[index]
        track: [track::RowId; VecCol<track::RowId>],
    }
}
impl Tracker for follow::track_track_events {
    fn cleared(&mut self, universe: &Universe) {
        follow::write(universe).clear();
    }
    fn track(&mut self, universe: &Universe, deleted: &[usize], added: &[usize]) {
        let follow = follow::write(universe);
        for delete in deleted {
        }
    }
}


#[test]
fn test() {
    TEST.register();
    track::register();
    follow::register();
    let universe = &Universe::new(&[TEST]);
}
