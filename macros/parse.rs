use syntex_syntax::ast::{MetaItem, MetaItemKind, LitKind, NestedMetaItemKind};
use syntex_syntax::symbol::keywords as keyword;
use syntex_syntax::parse::parser::Parser;
use syntex_syntax::parse::token::{Token, DelimToken, BinOpToken};
use syntex_syntax::parse::common::SeqSep;
use syntex_syntax::diagnostics::plugin::DiagnosticBuilder;

use super::table::{Table, Col, TableKind};
#[allow(unused_imports)]
use super::{warn, error};

macro_rules! err {
    ($parser:expr, $($args:tt),*) => {{
        return Err($parser.sess.span_diagnostic.struct_span_err($parser.span, &format!($($args),*)));
    }}
}

/*
 * table! {
 *     #[some_attribute]
 *     pub [domain_name/table_name] {
 *         observing: SegCol<::watchers::MyTable::RowId>,
 *         position: VecCol<MyCoordinate>,
 *         color: SegCol<RgbHexColor>,
 *         is_active: BoolCol,
 *     }
 *
 *     impl {
 *         something_or_other;
 *     }
 * }
 *
 * */
pub fn parse_table<'a>(parser: &mut Parser<'a>) -> Result<Table, DiagnosticBuilder<'a>> {
    let commas = SeqSep::trailing_allowed(Token::Comma);
    let mut table = Table::new();

    fn meta_arg(attr: &MetaItem) -> String {
        if let MetaItemKind::NameValue(ref lit) = attr.node {
            if let LitKind::Str(ref sym, _) = lit.node {
                return format!("{}", sym.as_str());
            }
        }
        panic!("Attributes should be of the form #[name = \"value\"]")
    }

    // [#[attr]] [pub] DOMAIN_NAME::table_name { ... }
    for attr in parser.parse_outer_attributes()?.into_iter() {
        let attr_name = format!("{}", attr.value.name);
        match attr_name.as_str() {
            "kind" => table.set_kind(match meta_arg(&attr.value).as_str() {
                "append" => TableKind::Append,
                "consistent" => TableKind::Consistent,
                "bag" => TableKind::Bag,
                "list" => TableKind::List,
                "sorted" => TableKind::Sorted,
                e => err!(parser, "Unknown table kind {:?}", e),
            }),
            "row_id" => table.row_id = meta_arg(&attr.value),
            "row_derive" => if let MetaItemKind::List(items) = attr.value.node {
                for item in &items {
                    let item = &item.node;
                    if let &NestedMetaItemKind::MetaItem(MetaItem { ref name, node: MetaItemKind::Word, .. }) = item {
                        match format!("{}", name.as_str()).as_str() {
                            "Clone" => table.derive.clone = true,
                            "Copy" => table.derive.copy = true,
                            "Debug" => table.derive.debug = true,
                            _ => (),
                        }
                    }
                }
                table.row_derive.extend(items);
            },
            "save" => table.save = true,
            "version" => {
                table.version = str::parse(meta_arg(&attr.value).as_str()).unwrap();
            },
            "add_tracker" => if let MetaItemKind::NameValue(lit) = attr.value.node {
                // #[add_tracker = "expression"]
                let lit = &lit.node;
                if let LitKind::Str(sym, _) = lit {
                    table.add_trackers.push(format!("{}", sym.as_str()));
                }
            },
            _ => {
                // other attrs go on the module
                table.module_attrs.push(attr);
            },
        }
    }
    if table.kind.is_none() {
        err!(parser, "Table kind not set");
    }
    table.is_pub = parser.eat_keyword(keyword::Pub); // FIXME: Syn + parse visibility
    //parser.expect(&Token::Mod)?;
    parser.expect(&Token::OpenDelim(DelimToken::Bracket))?;
    table.domain = parser.parse_ident()?.to_string();
    parser.expect(&Token::BinOp(BinOpToken::Slash))?;
    table.name = parser.parse_ident()?.to_string();
    parser.expect(&Token::CloseDelim(DelimToken::Bracket))?;


    // Load structure
    let structure_block = parser.parse_token_tree()?;
    table.cols = {
        let mut parser = Parser::new(parser.sess, vec![structure_block], None, true);
        parser.expect(&Token::OpenDelim(DelimToken::Brace))?;
        parser.parse_seq_to_end(&Token::CloseDelim(DelimToken::Brace), commas, |parser| {
            // #[attrs] column_name: [ElementType; ColumnType<ElementType>],
            let mut indexed = false;
            let mut foreign = false;
            let mut foreign_auto = false;
            let mut sort_key = false;
            let attrs = parser.parse_outer_attributes()?
                .into_iter()
                .filter(|attr| {
                    match format!("{}", attr.value.name).as_str() {
                        "index" => indexed = true,
                        "foreign" => foreign = true,
                        "sort_key" => sort_key = true,
                        "foreign_auto" => {
                            foreign = true;
                            foreign_auto = true;
                        },
                        _ => return true,
                    }
                    false
                }).collect();
            let name = parser.parse_ident()?;
            parser.expect(&Token::Colon)?;
            parser.expect(&Token::OpenDelim(DelimToken::Bracket))?;
            let element = parser.parse_ty()?;
            parser.expect(&Token::Semi)?;
            let colty = parser.parse_ty()?;
            parser.expect(&Token::CloseDelim(DelimToken::Bracket))?;
            if sort_key {
                if let Some(ref existing) = table.sort_key {
                    panic!("#[sort_key] already set on {:?}", existing);
                }
                if table.kind != Some(TableKind::Sorted) {
                    panic!("#[sort_key] requires #[table_kind = \"sorted\"]");
                }
                table.sort_key = Some(name.clone());
            }
            Ok(Col {
                attrs,
                name,
                element,
                colty,
                indexed,
                foreign,
                foreign_auto,
            })
        })?
    };

    // What tokens remain?
    for t in parser.tts.iter() {
        err!(parser, "Unexpected tokens at end of `table!`: {:?}", t);
    }
    Ok(table)
}
