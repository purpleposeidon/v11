#![allow(dead_code)]

#[macro_use]
extern crate v11;
#[macro_use]
extern crate v11_macros;
extern crate rustc_serialize;


domain! { TEST }

table! {
    #[kind = "consistent"]
    #[save]
    #[row_derive(RustcEncodable, RustcDecodable, Debug, Clone)]
    [TEST/saveme] {
        foo: [i32; VecCol<i32>],
        bar: [bool; BoolCol],
    }
}





#[test]
fn test() {
    use v11::*;
    TEST.register();
    saveme::register();

    let universe = &Universe::new(&[TEST]);
    {
        let mut saveme = saveme::write(universe);
        saveme.push(saveme::Row {
            foo: 1,
            bar: false,
        });
        saveme.push(saveme::Row {
            foo: 2,
            bar: true,
        });
        for blah in saveme.iter() {
            println!("{:?}", saveme.get_row_ref(blah));
        }
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
        let mut saveme = saveme::write(universe);
        let j = json::Json::from_str(&j).unwrap();
        let mut inp = json::Decoder::new(j);
        saveme.decode_rows(&mut inp).unwrap();
        for blah in saveme.iter() {
            println!("{:?}", saveme.get_row_ref(blah));
        }
    }
}
