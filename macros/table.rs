use syntex_syntax::ast::{Ident, Ty, Attribute, NestedMetaItem};
use syntex_syntax::ptr::P;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TableKind {
    Append,
    Consistent,
    Bag,
    List,
    Sorted,
}

#[derive(Default, Debug)]
pub struct Derives {
    pub clone: bool,
    pub debug: bool,
    pub copy: bool,
}

#[derive(Debug)]
#[derive(Default)] // Don't actually use this, it's just to keep new() easy.
pub struct Table {
    // Header
    pub module_attrs: Vec<Attribute>,
    pub row_derive: Vec<NestedMetaItem>,
    pub is_pub: bool,
    pub domain: String,
    pub name: String,
    pub kind: Option<TableKind>,
    pub cols: Vec<Col>,
    pub add_trackers: Vec<String>,

    // Modifiers
    pub row_id: String,
    pub version: u32,
    pub save: bool,
    pub derive: Derives,
    pub sort_key: Option<Ident>,

    // Guarantees
    pub immutable: bool,
    pub sorted: bool,
    pub consistent: bool,
    pub secret: bool,
}
impl Table {
    pub(crate) fn hash_names(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hasher, Hash};
        let mut h = DefaultHasher::new();
        for col in &self.cols {
            col.name.to_string().hash(&mut h);
        }
        h.finish()
    }

    pub(crate) fn set_kind(&mut self, kind: TableKind) {
        if self.kind.is_some() { panic!("table kind already set"); }
        self.kind = Some(kind);
        match kind {
            TableKind::Append => { },
            TableKind::Consistent => {
                self.consistent = true;
                self.secret = false;
            },
            TableKind::Bag => {
                self.secret = true;
                panic!("Bags are NYI");
            },
            TableKind::List => {
                self.secret = true;
            },
            TableKind::Sorted => {
                self.sorted = true;
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
        if self.save {
            assert!(self.derive.clone);
        }
    }
    pub fn new() -> Self {
        Table {
            row_id: "usize".to_owned(),
            version: 0,
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
        if self.derive.copy && !self.derive.clone {
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
    pub foreign_auto: bool,
    // just use BTreeMap for now; might want HashMap later tho
}
