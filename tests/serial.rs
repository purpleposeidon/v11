#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;


domain! { TEST }

table! {
    #[kind = "consistent"]
    #[save]
    #[row_derive(Serialize, Deserialize, Debug, Clone)]
    [TEST/saveme] {
        foo: [i32; VecCol<i32>],
        bar: [bool; BoolCol],
    }
}

table! {
    #[kind = "consistent"]
    #[save]
    #[row_derive(Clone, Debug)]
    [TEST/extra] {
        #[foreign_auto]
        #[index]
        user: [::saveme::RowId; VecCol<::saveme::RowId>],
    }
}

property! { static TEST/JSON_OUT: String }

use v11::event::{FallbackHandler, Event};
use v11::tracking::SelectAny;
use v11::tables::GenericTable;
use v11::Universe;
use std::sync::RwLock;

struct DumpSelection;
impl FallbackHandler for DumpSelection {
    fn needs_sort(&self, _gt: &RwLock<GenericTable>) -> bool { false }
    fn handle(&self, universe: &Universe, gt: &RwLock<GenericTable>, _event: Event, rows: SelectAny) {
        println!("Handling!");
        // This is a bit silly; with json you'd want it by row rather than by column.
        let mut out = universe[JSON_OUT].write().unwrap();
        let gt = gt.read().unwrap();
        use v11::serial::TableSelectionSer;
        *out += &serde_json::to_string_pretty(&TableSelectionSer::from(&*gt, &rows)).unwrap();
    }
}


#[test]
fn test() {
    use v11::*;
    TEST.register();
    JSON_OUT.register();
    saveme::register();
    extra::register();

    let mut universe = Universe::new(&[TEST]);
    universe.event_handlers.add(event::SAVE, Box::new(DumpSelection) as Box<FallbackHandler>);
    let universe = &universe;
    {
        let mut saveme = saveme::write(universe);
        saveme.push(saveme::Row {
            foo: 100,
            bar: false,
        });
        let mid = saveme.push(saveme::Row {
            foo: 200,
            bar: true,
        });
        saveme.push(saveme::Row {
            foo: 300,
            bar: true,
        });
        for blah in saveme.iter() {
            println!("{:?}", saveme.get_row_ref(blah));
        }
        saveme.flush(universe, event::CREATE);
        let mut extra = extra::write(universe);
        extra.push(extra::Row {
            user: mid,
        });
    }
    let json1: String;
    let json2: String;
    {
        let saveme = saveme::read(universe);
        println!("gonna save");
        saveme.select_all(universe, event::SAVE);
        {
            let mut json_out = universe[JSON_OUT].write().unwrap();
            json1 = json_out.clone();
            json_out.clear();
        }
        {
            let extra = extra::read(universe);
            extra.select_all(universe, event::SAVE);
            json2 = universe[JSON_OUT].read().unwrap().clone();
        }
        println!("saveme json:");
        println!("{}", json1);
        println!("--End--");
        println!("extra json:");
        println!("{}", json2);
        println!("--End--");
        {
            let alternia = &Universe::new(&[TEST]);
            let mut saveme2 = saveme::write(alternia);
            let deserializer = &mut serde_json::Deserializer::from_str(&json1);
            saveme2.deserialize(deserializer).unwrap();
            println!("Okay, we reconstitute it:");
            for i in saveme2.iter() {
                println!("{:?}", saveme2.get_row_ref(i));
            }
            println!("--End--");
            saveme2.flush(alternia, event::CREATE);
        }
    };
    {
        universe.set(JSON_OUT, String::new());
        let saveme = saveme::read(universe);
        println!("Gonna select/save just one row!");
        saveme.select_rows(universe, event::SAVE, true, Some(::saveme::FIRST).into_iter());
        println!("json saved:");
        let json = universe[JSON_OUT].read().unwrap();
        println!("{}", json);
        println!("--End--");
    }
    {
        println!("Clearing the tables");
        {
            let mut saveme = saveme::write(universe);
            saveme.clear();
            saveme.flush(universe, event::DELETE);
            let mut extra = extra::write(universe);
            extra.clear();
            extra.flush(universe, event::DELETE);
        }
        println!("Adding some new rows");
        {
            let mut saveme = saveme::write(universe);
            for foo in 0..5 {
                saveme.push(saveme::Row {
                    foo,
                    bar: foo % 2 == 0,
                });
            }
            saveme.flush(universe, event::CREATE);
        }
        println!("Regenerating everything from the saved json!");
        {
            let mut saveme = saveme::write(universe);
            let deserializer = &mut serde_json::Deserializer::from_str(&json1);
            saveme.deserialize(deserializer).unwrap();
            saveme.flush(universe, event::DESERIALIZE);

            let mut extra = extra::write(universe);
            let deserializer = &mut serde_json::Deserializer::from_str(&json2);
            extra.deserialize(deserializer).unwrap();
            extra.flush(universe, event::DESERIALIZE);
        }
        println!("What we have:");
        {
            let saveme = saveme::read(universe);
            for row in saveme.iter() {
                println!("{:?}", saveme.get_row(row));
            }
            let extra = extra::read(universe);
            for row in extra.iter() {
                println!("{:?}", extra.get_row(row));
            }
        }
    }
    /*{
        {
            let mut saveme = saveme::write(universe);
            saveme.clear();
            saveme.flush(universe, event::DELETE);
        }
        let mut saveme = saveme::write(universe);
        let j = &mut ::serde_json::de::Deserializer::from_str(&json);
        saveme.deserialize(j).unwrap();
        let mut n = 0;
        for blah in saveme.iter() {
            println!("{:?}", saveme.get_row_ref(blah));
            n += 1;
        }
        assert_eq!(n, 1);
        println!("Okay?");
        saveme.flush(universe, event::CREATE);
    }*/
}
