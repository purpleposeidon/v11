use syntex_syntax::parse::parser::Parser;
use syntex_syntax::parse::token::{Token, DelimToken, BinOpToken, Lit};
use syntex_syntax::parse::common::SeqSep;
use syntex_syntax::parse::PResult;
use syntex_syntax::tokenstream::TokenTree;
use syntex_syntax::symbol::keywords as keyword;
use syntex_syntax::diagnostics::plugin::DiagnosticBuilder;
use syntex_syntax::print::pprust as pp;

use super::table::{Table, Col};
#[allow(unused_imports)]
use super::{warn, error};

macro_rules! err {
    ($parser:expr, $($args:tt),*) => {
        return Err($parser.sess.span_diagnostic.struct_span_err($parser.span, &format!($($args),*)));
    }
}

/*
 * table! {
 *     #[some_attribute]
 *     pub domain_name/table_name {
 *         observing: SegCol<::watchers::MyTable::RowId>,
 *         position: VecCol<MyCoordinate>,
 *         color: SegCol<RgbHexColor>,
 *         is_active: BoolCol,
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
pub fn parse_table<'a>(parser: &mut Parser<'a>) -> Result<Table, DiagnosticBuilder<'a>> {
    let commas = SeqSep::trailing_allowed(Token::Comma);
    let mut table = Table::new();

    // [#[attr]] [pub] table_name { ... }
    table.attrs = parser.parse_outer_attributes()?;
    table.is_pub = parser.eat_keyword(keyword::Pub);
    table.domain = parser.parse_ident()?.to_string();
    parser.expect(&Token::BinOp(BinOpToken::Slash))?;
    table.name = parser.parse_ident()?.to_string();
    let structure_block = parser.parse_token_tree()?;

    // Load structure
    table.cols = {
        let mut parser = Parser::new(parser.sess, vec![structure_block], None, true);
        parser.expect(&Token::OpenDelim(DelimToken::Brace))?;
        parser.parse_seq_to_end(&Token::CloseDelim(DelimToken::Brace), commas, |parser| {
            // #[attrs] column_name: [ElementType; ColumnType<ElementType>],
            let attrs = parser.parse_outer_attributes().ok();
            let name = parser.parse_ident()?;
            parser.expect(&Token::Colon)?;
            parser.expect(&Token::OpenDelim(DelimToken::Bracket))?;
            let element = parser.parse_ty()?;
            parser.expect(&Token::Semi)?;
            let colty = parser.parse_ty()?;
            parser.expect(&Token::CloseDelim(DelimToken::Bracket))?;
            Ok(Col {
                attrs,
                name,
                element,
                colty,
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
            } else if name == "NoDebug" {
                table.debug = false;
            } else if name == "NoComplexMut" {
                table.no_complex_mut = true;
            } else if name == "NoCopy" {
                table.copy = false;
                table.no_complex_mut = true;
            } else if name == "NoClone" {
                table.copy = false;
                table.clone = false;
                table.no_complex_mut = true;
            } else if name == "Track" {
                table.track_changes = true;
            } else if name == "GenericSort" {
                // add a type-parameterized sort function
                table.generic_sort = true;
            } else if name == "SortBy" {
                // adds a function to sort a table by the given column
                // multiple individual calls
                let args = parse_arglist(parser)?;
                table.sort_by.extend(args);
            } else if name == "Merge" {
                let mut args = parse_arglist(parser)?;
                assert_eq!(args.len(), 1);
                table.merge = Some(args.pop().unwrap());
            } else if name == "FreeList" {
                // add a list of unused RowIds
                table.free_list = true;
            } else if name == "Save" {
                table.save = true;
            } else if name == "Static" {
                table.static_data = true;
            } else if name == "Version" || name == "Legacy" {
                // Version(1);
                parser.expect(&Token::OpenDelim(DelimToken::Paren))?;
                let version = parser.bump_and_get();
                parser.expect(&Token::CloseDelim(DelimToken::Paren))?;
                table.version = if let Token::Literal(Lit::Integer(name), _) = version {
                    use std::str::FromStr;
                    match usize::from_str(&name.as_str()) {
                        Ok(v) => v,
                        Err(_) => err!(parser, "Failed to parse version number {:?}", name),
                    }
                } else {
                    err!(parser, "Expected integer version, not {:?}", version);
                };
                if name == "Legacy" {
                    table.static_data = true;
                }
            } else {
                err!(parser, "Unknown modifier {:?}", name);
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
            what @ _ => {
                err!(parser, "Expected module code, got: {:?}", what);
            },
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
