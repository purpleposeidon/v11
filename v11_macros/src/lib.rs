#![recursion_limit = "1024"] // quote :D

extern crate syntex;
extern crate syntex_syntax;
#[macro_use]
extern crate quote;
extern crate walkdir;
extern crate rustfmt;

use std::path::Path;
use std::fs::File;
use std::ffi::OsStr;
use std::path::PathBuf;
use walkdir::WalkDir;

// FIXME: Modularize: lib.rs, table.rs, parse.rs, write.rs

#[allow(dead_code)]
fn warn<S: AsRef<str>>(msg: S) {
    for line in msg.as_ref().split("\n") {
        println!("cargo:warning={}", line);
    }
}

#[allow(dead_code)]
fn error<S: AsRef<str> + Clone>(msg: S) -> ! {
    // How to give error? panic's very unfriendly.
    warn(msg.clone());
    panic!("{}", msg.as_ref())
}

struct TmpFile {
    path: PathBuf,
}
impl TmpFile {
    fn new() -> Self {
        let path = ::std::env::temp_dir().join("v11_build_null");
        TmpFile {
            path: path,
        }
    }
}
impl Drop for TmpFile {
    fn drop(&mut self) {
        ::std::fs::remove_file(&self.path).ok();
    }
}

/// Generate the table code for the given crate. The typical build script calls this.
pub fn process_crate(crate_name: &str) {
    let out_dir = ::std::env::var("OUT_DIR").unwrap();
    let dst = Path::new(&out_dir).join("v11_generated_tables");
    process(crate_name, &Path::new("src/"), &dst);
}

/// Add the table expander to an existing `syntex::Registry`.
pub fn add_expander<P: AsRef<Path>>(registry: &mut syntex::Registry, output: P) {
    let expander = TableExpander::new(output);
    registry.add_macro("table", expander);
}

/// Recursively process each Rust source file in the input directory,
/// writing any table module definitions to the output directory.
pub fn process(crate_name: &str, input: &Path, output: &Path) {
    warn(&format!("{}: {:?} -> {:?}", crate_name, input, output));
    // We can't use `cargo:rerun-if-changed` because a new table could
    // be defined anywhere.
    assert!(input.is_dir(), "Input path is not a directory: {:?}", input);
    ::std::fs::create_dir_all(output).expect("Could not create output directory");
    let tmp = TmpFile::new();
    let dot_rs = Some(OsStr::new("rs"));
    // FIXME: temp file, remove? Hmm.
    // Kinda wasteful.
    for entry in WalkDir::new(input).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() { continue; }
        let source = entry.path();
        if source.extension() != dot_rs { continue; }
        warn(&format!("process: {:?}", source));
        let mut registry = syntex::Registry::new();
        add_expander(&mut registry, output);
        registry.expand(crate_name, source, &tmp.path).ok();
    }
}

use syntex_syntax::parse::parser::Parser;
use syntex_syntax::parse::token::{Token, DelimToken};
use syntex_syntax::parse::common::SeqSep;
use syntex_syntax::parse::PResult;
use syntex_syntax::ast::{Ident, Ty, Attribute};
use syntex_syntax::tokenstream::TokenTree;
use syntex_syntax::ext::quote::rt::Span;
use syntex_syntax::ext::base::{TTMacroExpander, ExtCtxt, MacResult};
use syntex_syntax::symbol::keywords as keyword;
use syntex_syntax::diagnostics::plugin::DiagnosticBuilder;
use syntex_syntax::ptr::P;
use syntex_syntax::print::pprust as pp;


pub struct TableExpander {
    /// A directory where table-modules are written to.
    out: PathBuf,
}
impl TableExpander {
    pub fn new<P: AsRef<Path>>(p: P) -> Self {
        TableExpander {
            out: p.as_ref().to_path_buf()
        }
    }
}
impl TTMacroExpander for TableExpander {
    fn expand<'cx>(
        &self,
        ecx: &'cx mut ExtCtxt,
        _span: Span,
        token_tree: &[TokenTree],
    ) -> Box<MacResult + 'cx> {
        let mut tokens = Vec::new();
        tokens.extend_from_slice(token_tree);
        let mut parser = Parser::new(ecx.parse_sess, tokens, None, false);
        let table = match parse_table(&mut parser) {
            Ok(t) => t,
            Err(m) => {
                error(&format!("{}", m.message()));
                // m.emit();
                // return DummyResult::any(span);
            },
        };
        if let Some(err) = table.validate() {
            error(err);
        }
        let path = self.out.join(&format!("{}.rs", table.name));
        {
            let out = File::create(&path);
            output::write_out(table, out.expect("Unable to open table module output file!")).expect("Write failed!");
        }
        let _ = rustfmt::run(rustfmt::Input::File(path), &rustfmt::config::Config {
            write_mode: rustfmt::config::WriteMode::Overwrite,
            .. Default::default()
        });

        use syntex_syntax::ext::base::MacEager;
        Box::new(MacEager::default()) as Box<MacResult>
    }
}

#[derive(Debug, Default)]
pub struct Table {
    // Header
    attrs: Vec<Attribute>,
    is_pub: bool,
    name: String,
    cols: Vec<Col>,

    // Modifiers
    debug: bool,
    version: usize,
    row_id: String,
    track_modify: bool,
    track_rm: bool,
    sync_rm: Option<String>,
    free_list: bool,
    encode: Vec<Serializer>,
    decode: Vec<Serializer>,
    cascade_deletions: Vec<String>,
    sort_by: Vec<String>,
    static_data: bool,

    // module
    mod_code: Option<String>,
}
impl Table {
    fn validate(&self) -> Option<&str> {
        if self.cols.is_empty() {
            return Some("No columns");
        }
        if self.static_data {
            if {
                self.track_modify
                || self.track_rm
                || self.sync_rm.is_some()
                || self.free_list
                || !self.cascade_deletions.is_empty()
                || !self.sort_by.is_empty()
                } {
                return Some("static tables shouldn't have modification features");
            }
        }
        None
    }
}

#[derive(Debug)]
struct Col {
    attrs: Option<Vec<Attribute>>,
    name: Ident,
    element: P<Ty>,
    colty: P<Ty>,
}

#[derive(Debug, Copy, Clone)]
enum Serializer {
    Rustc,
    Serde,
}


/*
 * table! {
 *     #[some_attribute]
 *     pub table_name {
 *         position: VecCol<MyCoordinate>,
 *         color: SegCol<RgbHexColor>,
 *         is_active: BitCol,
 *         observing: SegCol<::watchers::MyTable::RowId>,
 *     }
 *
 *     impl {
 *         something_or_other;
 *     }
 *     
 *     mod {
 *         use my_prelude::*;
 *
 *         impl Read<'a> {
 *             fn pretty_print(&self) { }
 *         }
 *     }
 * }
 *
 * */
fn parse_table<'a>(mut parser: &mut Parser<'a>) -> Result<Table, DiagnosticBuilder<'a>> {
    let commas = SeqSep {
        sep: Some(Token::Comma),
        trailing_sep_allowed: true,
    };
    let mut table: Table = Table::default();
    // Real defaults
    table.row_id = "usize".to_owned();

    // [#[attr]] [pub] table_name { ... }
    table.attrs = parser.parse_outer_attributes()?;
    table.is_pub = parser.eat_keyword(keyword::Pub);
    table.name = parser.parse_ident()?.to_string();
    let structure_block = parser.parse_token_tree()?;

    // Load structure
    table.cols = {
        let mut parser = Parser::new(parser.sess, vec![structure_block], None, true);
        parser.expect(&Token::OpenDelim(DelimToken::Brace))?;
        parser.parse_seq_to_end(&Token::CloseDelim(DelimToken::Brace), commas, |parser| {
            // column_name: [ElementType; ColumnType<ElementType>],
            let attrs = parser.parse_outer_attributes().ok();
            let name = parser.parse_ident()?;
            parser.expect(&Token::Colon)?;
            parser.expect(&Token::OpenDelim(DelimToken::Bracket))?;
            let element = parser.parse_ty()?;
            parser.expect(&Token::Semi)?;
            let colty = parser.parse_ty()?;
            parser.expect(&Token::CloseDelim(DelimToken::Bracket))?;
            Ok(Col {
                attrs: attrs,
                name: name,
                element: element,
                colty: colty,
            })
        })?
    };

    fn parse_arglist<'a>(parser: &mut Parser<'a>) -> PResult<'a, Vec<String>> {
        if !parser.eat(&Token::OpenDelim(DelimToken::Paren)) { return Ok(vec![]); }
        let commas = SeqSep {
            sep: Some(Token::Comma),
            trailing_sep_allowed: true,
        };
        parser.parse_seq_to_end(
            &Token::CloseDelim(DelimToken::Paren),
            commas,
            |x| Ok(pp::ident_to_string(Parser::parse_ident(x)?)),
        )
    }
    
    // [impl { ... }]
    // Configure modifiers
    if parser.eat_keyword(keyword::Impl) {
        let modifiers = parser.parse_token_tree()?;
        let mut parser = Parser::new(parser.sess, vec![modifiers], None, true);
        let parser = &mut parser;
        parser.expect(&Token::OpenDelim(DelimToken::Brace))?;
        loop {
            if parser.eat(&Token::CloseDelim(DelimToken::Brace)) {
                break;
            }
            let name = parser.parse_ident()?.name;
            if name == "RowId" {
                // RowId = usize
                parser.expect(&Token::Eq)?;
                table.row_id = pp::ty_to_string(&*parser.parse_ty()?);
            } else if name == "Debug" {
                table.debug = true;
            } else if name == "TrackRm" {
                table.track_rm = true;
            } else if name == "ForeignCascade" {
                // adds a 'cascade_foreign_deletions' fn that removes foreign keys whose parent has been deleted
                // a single vararg call
                // FIXME: no more custom-named tables
                let args = parse_arglist(parser)?;
                table.cascade_deletions.extend(args);
            } else if name == "TrackModify" {
                // keep a sparse & transient list of modified/created/removed rows
                table.track_modify = true;
            } else if name == "SortBy" {
                // adds a function to sort a table by the given column
                // multiple individual calls
                let args = parse_arglist(parser)?;
                table.sort_by.extend(args);
            } else if name == "FreeList" {
                // add a list of unused RowIds
                table.free_list = true;
            } else if name == "Encode" || name == "Decode" {
                let out = if name == "Encode" {
                    &mut table.encode
                } else {
                    &mut table.decode
                };
                let modes = parse_arglist(parser)?;
                if modes.is_empty() {
                    out.push(Serializer::Rustc);
                } else {
                    for e in modes {
                        if e == "Rustc" {
                            out.push(Serializer::Rustc);
                        } else if e == "Serde" {
                            out.push(Serializer::Serde);
                        } else {
                            panic!("Unknown serializer {:?}", e);
                        }
                    }
                }
            } else if name == "Static" {
                table.static_data = true;
            } else {
                panic!("Unknown modifier {}", name);
            }
            parser.expect(&Token::Semi)?;
        }
    }

    // [mod { ... }]
    table.mod_code = if !parser.eat_keyword(keyword::Mod) {
        None
    } else {
        let got = match parser.parse_token_tree()? {
            TokenTree::Delimited(_, d) => pp::tts_to_string(&d.tts[..]),
            what @ _ => error(&format!("Expected module code, got: {:?}", what)),
        };
        Some(got)
    };


    // What tokens remain?
    for t in parser.tts.iter() {
        if let &(TokenTree::Delimited(_, ref d), _) = t {
            for t in d.tts.iter() {
                warn(&format!("{:?}", t));
            }
        } else {
            warn(&format!("{:?}", t));
        }
    }
    Ok(table)
}

mod output;
