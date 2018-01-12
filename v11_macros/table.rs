use syntex_syntax::ast::{Ident, Ty, Attribute};
use syntex_syntax::ptr::P;

#[derive(Debug, Copy, Clone)]
pub enum TableKind {
    Append,
    Public,
    Bag,
}

#[derive(Debug)]
#[derive(Default)] // Don't actually use this, it's just to keep new() easy.
pub struct Table {
    // Header
    pub module_attrs: Vec<Attribute>,
    pub is_pub: bool,
    pub domain: String,
    pub name: String,
    pub kind: Option<TableKind>,
    pub cols: Vec<Col>,

    // Modifiers
    pub row_id: String,
    pub debug: bool,
    pub copy: bool,
    pub clone: bool,
    pub version: usize,
    pub save: bool,

    // Guarantees
    pub immutable: bool,
    pub sorted: bool,
    pub consistent: bool,
    pub secret: bool,
}
impl Table {
    pub(crate) fn set_kind(&mut self, kind: TableKind) {
        if self.kind.is_some() { panic!("table kind already set"); }
        self.kind = Some(kind);
        match kind {
            TableKind::Append => { },
            TableKind::Public => {
                self.consistent = true;
                self.secret = false;
            },
            TableKind::Bag => {
                self.secret = true;
                panic!("Bags are NYI");
            },
        }
        for col in &self.cols {
            if col.indexed { panic!("Indexes are NYI"); }
        }
    }
    fn validate_guarantees(&self) {
        if self.immutable {
            assert!(!self.sorted);
            assert!(!self.secret);
        }
        if self.sorted {
            assert!(!self.immutable);
            assert!(!self.consistent);
        }
        if self.consistent {
            assert!(!self.sorted);
            assert!(!self.secret);
        }
    }
    pub fn new() -> Self {
        Table {
            debug: true,
            copy: true,
            clone: true,
            row_id: "usize".to_owned(),
            version: 1,
            .. Table::default()
        }
    }
    pub(crate) fn validate(&mut self) -> Option<&'static str> {
        self.validate_guarantees();
        if self.domain.is_empty() {
            return Some("No domain");
        }
        if self.name.is_empty() {
            return Some("No name");
        }
        if self.cols.is_empty() {
            return Some("No columns");
        }
        if self.copy && !self.clone {
            return Some("deriving copy, but not clone");
        }
        None
    }
}

#[derive(Debug)]
pub struct Col {
    pub attrs: Vec<Attribute>,
    pub name: Ident,
    pub element: P<Ty>,
    pub colty: P<Ty>,
    pub indexed: bool,
    pub foreign: bool,
    // just use BTreeMap for now; might want HashMap later tho
}
