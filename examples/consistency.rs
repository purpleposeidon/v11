#[macro_use] extern crate v11_macros;
#[macro_use] extern crate v11;

use v11::Universe;
use v11::event;

domain! { EXAMPLE }

table! {
    #[kind = "consistent"]
    #[row_derive(Clone, Debug)]
    [EXAMPLE/ships] {
        name: [String; VecCol<String>],
        cargo: [u8; VecCol<u8>],
    }
}

table! {
    #[kind = "consistent"]
    #[row_derive(Debug)]
    [EXAMPLE/sailors] {
        #[foreign_auto]
        #[index]
        ship: [ships::RowId; VecCol<ships::RowId>],
        name: [String; VecCol<String>],
    }
}


fn main() {
    EXAMPLE.register();
    ships::register();
    sailors::register();
    let universe = &Universe::new(&[EXAMPLE]);

    let boaty_mcboatface = {
        let mut ships = ships::write(universe);
        let mont_blanc = ships.push(ships::Row {
            name: "SS Mont-Blanc".into(),
            cargo: 11,
        });
        let lusitania = ships.push(ships::Row {
            name: "RMS Lusitania".into(),
            cargo: 237,
        });
        let titanic = ships.push(ships::Row {
            name: "RMS Titanic".into(),
            cargo: 42,
        });
        let boaty_mcboatface = ships.push(ships::Row {
            name: "Boaty McBoatface".into(),
            cargo: 24,
        });
        let mut sailors = sailors::write(universe);
        sailors.push(sailors::Row {
            ship: titanic,
            name: "Alice".into(),
        });
        sailors.push(sailors::Row {
            ship: boaty_mcboatface,
            name: "Bob".into(),
        });
        sailors.push(sailors::Row {
            ship: lusitania,
            name: "Charles".into(),
        });
        sailors.push(sailors::Row {
            ship: mont_blanc,
            name: "Dave".into(),
        });
        sailors.close();
        ships.flush(universe, event::CREATE);
        boaty_mcboatface
    };
    show(universe);
    {
        let mut ships = ships::write(universe);
        println!();
        println!("The Boaty McBoatface is sinking! Oh, the humanity!");
        println!();
        ships.delete(boaty_mcboatface);
        ships.flush(universe, event::DELETE);
    }
    show(universe);
}

fn show(universe: &Universe) {
    let ships = ships::read(universe);
    let sailors = sailors::read(universe);
    for ship in ships.iter() {
        println!("{:?} = {:?}", ship, ships.get_row(ship));
    }
    for sailor in sailors.iter() {
        println!("{:?} = {:?}", sailor, sailors.get_row_ref(sailor));
    }
}
