#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;
extern crate rustc_serialize;

use v11::*;
use v11::tables::RowRange;



// FIXME: Test sorting.

// FIXME: Can this go into "tests/" as is standard? Previous usage of build.rs probably made that
// difficult.

domain! { pub TEST }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Blah;

table! {
    /// Can we document the table?
    [TEST/elements] {
        bits: [bool; BoolCol],
        bytes: [u8; VecCol<u8>],
        blah: [Blah; VecCol<Blah>],
    }
}

table! {
    [TEST/arrays] {
        range: [RowRange<elements::RowId>; VecCol<RowRange<elements::RowId>>],
    }
    impl {
        Save;
    }
}

fn verse() -> Universe {
    TEST.register();
    elements::register();
    arrays::register();
    Universe::new(&[TEST])
}

#[test]
fn ranged() {
    let universe = &verse();
    let mut e = elements::write(universe);
    let mut a = arrays::write(universe);
    {

        for n in 0..3 {
            let start = e.push(elements::Row {
                bits: true,
                bytes: 42,
                blah: Blah,
            });
            let mut end = start;
            for _ in 0..n {
                end = e.push(elements::Row {
                    bits: false,
                    bytes: 24,
                    blah: Blah,
                });
            }
            a.push(arrays::Row {
                range: RowRange {
                    start: start,
                    end: end.next(),
                },
            });
        }
    }

    for row in a.range() {
        println!("{:?}: {:?}", row, a.range[row]);
        for erow in a.range[row] {
            println!("\t{}", e.bits[erow]);
        }
        // RowRange is copy, so this does the full iteration twice.
        println!("{:?}: {:?}", row, a.range[row]);
        for erow in a.range[row] {
            println!("\t{}", e.bits[erow]);
        }
    }
}
