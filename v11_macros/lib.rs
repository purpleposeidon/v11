#![recursion_limit = "1024"] // quote :D

#[macro_use]
extern crate procedural_masquerade;
extern crate proc_macro;
extern crate syntex;
extern crate syntex_syntax;
#[macro_use]
extern crate quote;
extern crate rustfmt;


mod parse;
mod table;
mod output;



define_proc_macros! {
    #[allow(non_snake_case)]
    pub fn __v11_internal_table(macro_args: &str) -> String {
        use syntex_syntax::parse::{ParseSess, new_parser_from_source_str};
        let macro_args = macro_args.to_owned();
        let sess = ParseSess::new();
        let mut parser = new_parser_from_source_str(&sess, "<table! macro>".to_owned(), macro_args);
        let mut table = match ::parse::parse_table(&mut parser) {
            Err(msg) => {
                println!("{:?}", msg);
                return String::new();
            },
            Ok(t) => t,
        };
        if let Some(err) = table.validate() {
            error(err);
            return String::new();
        }

        let mut ret = Vec::new();
        ::output::write_out(table, &mut ret).unwrap();
        String::from_utf8(ret).unwrap()
    }
}



use std::fmt::Display;

#[allow(dead_code)]
fn warn<S: Display>(msg: S) {
    println!("{}", msg);
    /*
    for line in msg.as_ref().split("\n") {
        println!("cargo:warning={}", line);
    }*/
}

#[allow(dead_code)]
fn error<S: Display>(msg: S) {
    warn(msg);
}
