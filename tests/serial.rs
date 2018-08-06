#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate rustc_serialize;
extern crate serde_json;


domain! { TEST }

table! {
    #[kind = "consistent"]
    #[save]
    #[row_derive(RustcEncodable, RustcDecodable, Serialize, Deserialize, Debug, Clone)]
    [TEST/saveme] {
        foo: [i32; VecCol<i32>],
        bar: [bool; BoolCol],
    }
}

property! { static TEST/JSON_OUT: String }

use v11::event::{FallbackHandler, Event};
use v11::tracking::SelectAny;
use v11::tables::GenericTable;
use v11::Universe;
use std::sync::RwLock;

struct FullDump;
impl FallbackHandler for FullDump {
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

    let mut universe = Universe::new(&[TEST]);
    universe.event_handlers.add(event::SAVE, Box::new(FullDump) as Box<FallbackHandler>);
    let universe = &universe;
    let expect_len;
    {
        let mut saveme = saveme::write(universe);
        saveme.push(saveme::Row {
            foo: 100,
            bar: false,
        });
        saveme.push(saveme::Row {
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
        expect_len = saveme.len();
        saveme.flush(universe, event::CREATE);
    }
    {
        let saveme = saveme::read(universe);
        println!("gonna save");
        saveme.select_all(universe, event::SAVE);
        println!("json saved:");
        let json = universe[JSON_OUT].read().unwrap();
        println!("{}", json);
        println!("--End--");
        {
            let alternia = &Universe::new(&[TEST]);
            let mut saveme2 = saveme::write(alternia);
            let deserializer = &mut serde_json::Deserializer::from_str(&json);
            saveme2.deserialize(deserializer).unwrap();
            println!("Okay, we reconstitute it:");
            for i in saveme2.iter() {
                println!("{:?}", saveme2.get_row_ref(i));
            }
            println!("--End--");
        }
    }
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
    use rustc_serialize::json;
    let mut j = String::new();
    {
        let saveme = saveme::read(universe);
        let mut out = json::Encoder::new_pretty(&mut j);
        saveme.encode_rows(&mut out).unwrap();
    }
    println!("{}", j);
    {
        {
            let mut saveme = saveme::write(universe);
            saveme.clear();
            saveme.flush(universe, event::DELETE);
        }
        let mut saveme = saveme::write(universe);
        let j = json::Json::from_str(&j).unwrap();
        let mut inp = json::Decoder::new(j);
        saveme.decode_rows(&mut inp).unwrap();
        let mut n = 0;
        for blah in saveme.iter() {
            println!("{:?}", saveme.get_row_ref(blah));
            n += 1;
        }
        assert_eq!(n, expect_len);
    }
}
