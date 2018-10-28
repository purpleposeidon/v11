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
    [TEST/delete_ships] {
        name: [Name; VecCol<Name>],
    }
}


#[test]
fn test() {
    TEST.register();
    delete_ships::register();
    let universe = &Universe::new(&[TEST]);

    let boaty_mcboatface = {
        let mut delete_ships = delete_ships::write(universe);
        let _titanic = delete_ships.push(delete_ships::Row {
            name: "RMS Titanic",
        });
        let boaty_mcboatface = delete_ships.push(delete_ships::Row {
            name: "Boaty McBoatface",
        });
        let _lusitania = delete_ships.push(delete_ships::Row {
            name: "RMS Lusitania",
        });
        let _mont_blanc = delete_ships.push(delete_ships::Row {
            name: "SS Mont-Blanc",
        });
        delete_ships.flush(universe, event::CREATE);
        boaty_mcboatface
    };
    {
        let mut delete_ships = delete_ships::write(universe);
        println!("The Boaty McBoatface is sinking! Oh, the humanity!");
        delete_ships.delete(boaty_mcboatface);
        delete_ships.flush(universe, event::DELETE);
    }
    {
        let delete_ships = delete_ships::read(universe);
        let mut tcount = 0;
        println!("{}", delete_ships.len());
        for ship in delete_ships.iter() {
            println!("{:?}", delete_ships.get_row(ship));
            tcount += 1;
        }
        assert_eq!(tcount, 3);
    }
}
