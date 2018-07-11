#![recursion_limit = "1024"] // quote :D

#[macro_use]
extern crate procedural_masquerade;
extern crate proc_macro;
// FIXME: Switch to syn. This can wait for macros2.0.
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
                let mut diagnostic = msg.into_diagnostic();
                diagnostic.cancel();
                panic!("{}", diagnostic.message());
            },
            Ok(t) => t,
        };
        if let Some(err) = table.validate() {
            error(err);
            return String::new();
        }

        use std::env::var as env;
        let output_path = format!(
            "{}/target/v11_dump/{}_{}.rs",
            env("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"),
            table.domain,
            table.name,
        ); // FIXME: There doesn't seem to be a proper way of doing this.
        let output_path = ::std::path::Path::new(&output_path);
        let dump = match env("V11_MACRO_DUMP").as_ref().map(String::as_str) {
            Ok("*") => true,
            Ok("0") | Ok("") => false,
            Ok(n) => n == table.name,
            _ => true,
        };
        let mut ret = Vec::new();
        ::output::write_out(table, &mut ret).unwrap();
        let ret = String::from_utf8(ret).unwrap();
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
            use std::fs;
            use std::io::Write;
            fs::create_dir_all(output_path.parent().expect("no parent")).ok();
            let mut out = fs::File::create(output_path).expect("unable to create dump file");
            out.write(formatted.as_bytes()).expect("dump failed");
            format!("include!({:?});", output_path)
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
