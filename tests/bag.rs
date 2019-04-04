#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;


domain! { TEST }


table! {
    #[kind = "bag"]
    #[row_derive(Debug, Clone)]
    [TEST/hello_there] {
        foo: [i32; SegCol<i32>],
    }
}

use v11::Universe;

#[test]
fn compiles() {
    TEST.register();
    hello_there::register();
    let universe = &Universe::new(&[TEST]);
    {
        let mut ht = hello_there::write(universe);
        ht.push(hello_there::Row {
            foo: 0,
        });
        ht.push(hello_there::Row {
            foo: 1,
        });
        ht.push(hello_there::Row {
            foo: 2,
        });
    }
    {
        let ht = hello_there::read(universe);
        assert_eq!(ht.foo[hello_there::RowId::from_usize(2)], 2);
    }
    {
        let mut ht = hello_there::write(universe);
        ht.delete(hello_there::RowId::from_usize(1));
    }
    {
        println!("listing");
        let ht = hello_there::read(universe);
        for i in ht.iter() {
            println!("{:?}", ht.get_row(i));
        }
    }
    {
        let mut ht = hello_there::write(universe);
        for (_table, i) in ht.iter_mut() {
            i.delete();
            break;
        }
    }
    {
        println!("listing");
        let ht = hello_there::read(universe);
        for i in ht.iter() {
            println!("{:?}", ht.get_row(i));
        }
    }
    {
        let mut ht = hello_there::write(universe);
        ht.clear();
    }
    {
        println!("listing");
        let ht = hello_there::read(universe);
        for i in ht.iter() {
            println!("{:?}", ht.get_row(i));
        }
        println!("...is empty, yes?");
    }
}
