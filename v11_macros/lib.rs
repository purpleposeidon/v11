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

use syntex_syntax::ast::{Ident, Ty};
use syntex_syntax::ptr::P;

pub(crate) struct ConstTokens {
    _event_name: Ident,
    _event_element: P<Ty>,
    _event_colty: P<Ty>,
}


define_proc_macros! {
    #[allow(non_snake_case)]
    pub fn __v11_internal_table(macro_args: &str) -> String {
        use syntex_syntax::parse::{ParseSess, new_parser_from_source_str};
        let macro_args = macro_args.to_owned();
        let sess = ParseSess::new();
        let const_tokens = {
            let parser = |src: &str| {
                let src = src.to_owned();
                new_parser_from_source_str(&sess, "<table! const tokens>".to_owned(), src)
                    .parse_ty().expect("const src parse")
            };
            ConstTokens {
                _event_name: Ident::from_str("_events"),
                _event_element: parser("Event<RowId>"),
                _event_colty: parser("VecCol<Event<RowId>>"),
            }
        };
        let mut parser = new_parser_from_source_str(&sess, "<table! macro>".to_owned(), macro_args);
        let mut table = match ::parse::parse_table(&mut parser) {
            Err(msg) => {
                println!("{:?}", msg);
                return String::new();
            },
            Ok(t) => t,
        };
        if let Some(err) = table.validate(&const_tokens) {
            error(err);
            return String::new();
        }

        let mut ret = Vec::new();
        ::output::write_out(table, &mut ret).unwrap();
        let ret = String::from_utf8(ret).unwrap();
        let dump = ::std::env::var("V11_MACRO_DUMP").is_ok();
        if dump {
            // formatting
            let formatted = {
                use rustfmt::*;
                let mut formatted = Vec::new();
                let (_, mut filemap, _) = format_input(
                    Input::Text(ret),
                    &config::Config::default(),
                    Some(&mut formatted),
                ).unwrap();
                let out = filemap.pop().unwrap();
                out.1.to_string()
            };
            println!("==== formatted ==== ");
            println!("{}", formatted);
            formatted
        } else {
            ret
        }
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
