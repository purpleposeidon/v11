#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;

extern crate rustc_serialize;


domain! { TEST }
use v11::Universe;
use v11::tracking::Tracker;

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
        #[foreign]
        #[index]
        ship: [ships::RowId; VecCol<ships::RowId>],
        name: [Name; VecCol<Name>],
    }
}

impl Tracker for sailors::track_ship_events {
    fn cleared(&mut self, universe: &Universe) {
        sailors::write(universe).clear();
    }
    fn track(&mut self, universe: &Universe, deleted_ships: &[usize], _added: &[usize]) {
        println!("deleted: {:?}", deleted_ships);
        let mut sailors = sailors::write(universe);
        sailors.track_ship_removal(deleted_ships);
    }
}


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
            name: "Carol",
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
            name: "Carol",
        });
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
        ships.flush(universe);
        boaty_mcboatface
    };
    {
        let mut ships = ships::write(universe);
        println!("oh no the Boaty McBoatface has sunk!!");
        ships.delete(boaty_mcboatface);
        ships.flush(universe);
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
