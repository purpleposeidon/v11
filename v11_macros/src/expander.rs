
use syntex_syntax::ext::base::{TTMacroExpander, ExtCtxt, MacResult, DummyResult};
use syntex_syntax::ext::quote::rt::Span;
use syntex_syntax::tokenstream::TokenTree;
use syntex_syntax::parse::parser::Parser;

use super::error;

use std::path::{Path, PathBuf};
use std::fs::File;

pub struct TableExpander {
    /// A directory where table-modules are written to.
    out: PathBuf,
    run_rust_format: bool,
}
impl TableExpander {
    pub fn new<P: AsRef<Path>>(p: P, run_rust_format: bool) -> Self {
        TableExpander {
            out: p.as_ref().to_path_buf(),
            run_rust_format: run_rust_format,
        }
    }
}
impl TTMacroExpander for TableExpander {
    fn expand<'cx>(
        &self,
        ecx: &'cx mut ExtCtxt,
        span: Span,
        token_tree: &[TokenTree],
    ) -> Box<MacResult + 'cx> {
        let mut tokens = Vec::new();
        tokens.extend_from_slice(token_tree);
        let mut parser = Parser::new(ecx.parse_sess, tokens, None, false);
        let mut table = match super::parse::parse_table(&mut parser) {
            Ok(t) => t,
            Err(mut m) => {
                m.emit();
                return DummyResult::any(span);
            },
        };
        if let Some(err) = table.validate() {
            error(format!("Problem with table {:?}/{:?}: {}", table.domain, table.name, err));
            return DummyResult::any(span);
        }
        let path = {
            let basedir = self.out.join(&table.domain);
            ::std::fs::create_dir_all(&basedir).expect("Could not create output directory");
            basedir.join(&format!("{}.rs", table.name))
        };
        {
            let out = File::create(&path);
            super::output::write_out(table, out.expect("Unable to open table module output file!")).expect("Write failed!");
        }
        if self.run_rust_format {
            // FIXME: Add a crate option?
            use rustfmt;
            use rustfmt::config::{Config, WriteMode};
            let _ = rustfmt::run(rustfmt::Input::File(path), &Config {
                write_mode: WriteMode::Overwrite,
                .. Default::default()
            });
        }

        use syntex_syntax::ext::base::MacEager;
        Box::new(MacEager::default()) as Box<MacResult>
    }
}



