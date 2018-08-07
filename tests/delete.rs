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


#[test]
fn test() {
    TEST.register();
    ships::register();
    let universe = &Universe::new(&[TEST]);

    let boaty_mcboatface = {
        let mut ships = ships::write(universe);
        let _titanic = ships.push(ships::Row {
            name: "RMS Titanic",
        });
        let boaty_mcboatface = ships.push(ships::Row {
            name: "Boaty McBoatface",
        });
        let _lusitania = ships.push(ships::Row {
            name: "RMS Lusitania",
        });
        let _mont_blanc = ships.push(ships::Row {
            name: "SS Mont-Blanc",
        });
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
        let mut tcount = 0;
        println!("{}", ships.len());
        for ship in ships.iter() {
            println!("{:?}", ships.get_row(ship));
            tcount += 1;
        }
        assert_eq!(tcount, 3);
    }
}
