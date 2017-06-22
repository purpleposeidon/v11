#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;

use v11::*;
use v11::tables::RowRange;



// FIXME: Test sorting.

// FIXME: Can this go into "tests/" as is standard? Previous usage of build.rs probably made that
// difficult.

domain! { pub TEST }
table! {
    /// Can we document the table?
    [TEST/elements] {
        bits: [bool; BoolCol],
        bytes: [u8; VecCol<u8>],
    }
}

table! {
    [TEST/arrays] {
        range: [RowRange<elements::RowId>; VecCol<RowRange<elements::RowId>>],
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
            });
            let mut end = start;
            for _ in 0..n {
                end = e.push(elements::Row {
                    bits: false,
                    bytes: 24,
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
    }
    for row in a.range() {
        println!("{:?}: {:?}", row, a.range[row]);
        for erow in a.range[row] {
            println!("\t{}", e.bits[erow]);
        }
    }
}
