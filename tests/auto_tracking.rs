#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;


domain! { TEST }
use v11::Universe;
use v11::event;

type Name = &'static str;

table! {
    #[kind = "consistent"]
    #[row_derive(Clone, Debug)]
    [TEST/ships] {
        name: [Name; VecCol<Name>],
    }
}

table! {
    #[kind = "consistent"]
    #[row_derive(Debug)]
    [TEST/sailors] {
        #[foreign_auto]
        #[index]
        ship: [ships::RowId; VecCol<ships::RowId>],
        name: [Name; VecCol<Name>],
    }
}

//pub mod ships { include!("/tmp/b.rs"); }
//pub mod sailors { include!("/tmp/a.rs"); }

#[test]
fn test() {
    TEST.register();
    ships::register();
    sailors::register();
    let universe = &Universe::new(&[TEST]);

    let boaty_mcboatface = {
        let mut ships = ships::write(universe);
        let titanic = ships.push(ships::Row {
            name: "RMS Titanic",
        });
        let boaty_mcboatface = ships.push(ships::Row {
            name: "Boaty McBoatface",
        });
        let lusitania = ships.push(ships::Row {
            name: "RMS Lusitania",
        });
        let _mont_blanc = ships.push(ships::Row {
            name: "SS Mont-Blanc",
        });
        ships.live_flush(universe, event::CREATE);
        let mut sailors = sailors::write(universe);
        sailors.push(sailors::Row {
            ship: titanic,
            name: "Alice",
        });
        sailors.push(sailors::Row {
            ship: titanic,
            name: "Bob",
        });
        sailors.push(sailors::Row {
            ship: boaty_mcboatface,
            name: "Charles",
        });
        sailors.push(sailors::Row {
            ship: lusitania,
            name: "Alice",
        });
        sailors.push(sailors::Row {
            ship: lusitania,
            name: "Bob",
        });
        sailors.push(sailors::Row {
            ship: lusitania,
            name: "Charles",
        });
        sailors.live_flush(universe, event::CREATE);
        {
            let (mut ships, ship_iter) = ships.editing();
            for ship in ship_iter {
                println!("{:?}", ships.get_row(ship));
            }
            let (mut sailors, sailors_iter) = sailors.editing();
            for sailor in sailors_iter {
                println!("{:?}", sailors.get_row_ref(sailor));
            }
        }
        assert_eq!(ships.len(), 4);
        assert_eq!(sailors.len(), 6);
        sailors.close();
        ships.flush(universe, event::CREATE);
        boaty_mcboatface
    };
    {
        let mut ships = ships::write(universe);
        println!("The Boaty McBoatface is sinking! Oh, the humanity!");
        ships.delete(boaty_mcboatface);
        ships.flush(universe, event::DELETE);
    }
    {
        let ships = ships::read(universe);
        let sailors = sailors::read(universe);
        let mut tcount = 0;
        for ship in ships.iter() {
            println!("{:?}", ships.get_row(ship));
            tcount += 1;
        }
        let mut hcount = 0;
        for sailor in sailors.iter() {
            println!("{:?}", sailors.get_row_ref(sailor));
            hcount += 1;
        }
        assert_eq!(tcount, 3);
        assert_eq!(hcount, 5);
    }
}
