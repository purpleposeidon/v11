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
    #[row_derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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
        user: [saveme::RowId; VecCol<saveme::RowId>],
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
        let selection = gt
            .table
            .extract_serialization(universe, rows)
            .expect("table is not serializable!");
        {
            // Okay, this is a bit dumb. This example wants separate json blobs.
            // It could just serialize the tables manually...
            // But it's just an example.
            out.clear();
        }
        *out += &serde_json::to_string_pretty(&selection).unwrap();
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
    universe.event_handlers.add(event::SERIALIZE, Box::new(DumpSelection) as Box<FallbackHandler>);
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
        println!("Here's saveme:");
        for blah in saveme.iter() {
            println!("{:?}", saveme.get_row_ref(blah));
        }
        saveme.flush(universe, event::CREATE);
        let mut extra = extra::write(universe);
        extra.push(extra::Row {
            user: mid,
        });
        extra.flush(universe, event::CREATE);
        let extra = extra::read(universe);
        println!("Here's extra:");
        for blah in extra.iter() {
            println!("{:?}", extra.get_row_ref(blah));
        }
    }
    let json1: String;
    let json2: String;
    {
        let saveme = saveme::read(universe);
        println!("gonna save");
        saveme.select_all(universe, event::SERIALIZE);
        let saveme = saveme::read(universe);
        {
            let mut json_out = universe[JSON_OUT].write().unwrap();
            json1 = json_out.clone();
            json_out.clear();
        }
        {
            let extra = extra::read(universe);
            extra.select_all(universe, event::SERIALIZE);
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
            let extraction: saveme::Extraction = serde_json::from_str(&json1).unwrap();
            let saveme2 = saveme::write(alternia);
            saveme2.restore_extract(alternia, extraction, event::DESERIALIZE).unwrap();


            println!("Okay, we reconstitute it:");
            let saveme2 = saveme::read(alternia);
            for i in saveme2.iter() {
                println!("{:?}", saveme2.get_row_ref(i));
                assert_eq!(saveme2.get_row_ref(i), saveme.get_row_ref(i));
            }
            println!("--End--");
        }
    };
    {
        universe.set(JSON_OUT, String::new());
        let saveme = saveme::read(universe);
        println!("Gonna select/save just one row!");
        saveme.select_rows(universe, event::SERIALIZE, true, Some(crate::saveme::FIRST).into_iter());
        println!("json saved:");
        let json = universe[JSON_OUT].read().unwrap();
        println!("{}", json);
        println!("--End--");
    }
    {
        println!("Clearing the tables");
        {
            println!("clearing saveme");
            let mut saveme = saveme::write(universe);
            saveme.clear();
            saveme.flush(universe, event::DELETE);
            println!("clearing extra");
            let mut extra = extra::write(universe);
            extra.clear();
            extra.flush(universe, event::DELETE);
        }
        println!("Adding some extra new rows; hopefully we can cope with this!");
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
            let extraction: saveme::Extraction = serde_json::from_str(&json1).unwrap();
            let saveme = saveme::write(universe);
            saveme.restore_extract(universe, extraction, event::DESERIALIZE).unwrap();

            let extraction: extra::Extraction = serde_json::from_str(&json2).unwrap();
            let extra = extra::write(universe);
            extra.restore_extract(universe, extraction, event::DESERIALIZE).unwrap();
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
                assert_eq!(saveme.foo[extra.user[row]], 200);
            }
        }
    }
}
