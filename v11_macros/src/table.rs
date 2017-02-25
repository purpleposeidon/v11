use syntex_syntax::ast::{Ident, Ty, Attribute};
use syntex_syntax::ptr::P;

#[derive(Debug)]
#[derive(Default)] // Don't actually use this, it's just to keep new() easy.
pub struct Table {
    // Header
    pub attrs: Vec<Attribute>,
    pub is_pub: bool,
    pub name: String,
    pub cols: Vec<Col>,

    // Modifiers
    pub debug: bool,
    pub version: usize,
    pub row_id: String,
    pub track_modify: bool,
    pub track_rm: bool,
    pub sync_rm: Option<String>,
    pub free_list: bool,
    pub encode: Vec<Serializer>,
    pub decode: Vec<Serializer>,
    pub cascade_deletions: Vec<String>,
    pub generic_sort: bool,
    pub sort_by: Vec<String>,
    pub static_data: bool,

    // module
    pub mod_code: Option<String>,
}
impl Table {
    pub fn new() -> Self {
        Table {
            debug: true,
            row_id: "usize".to_owned(),
            version: 1,
            .. Table::default()
        }
    }
    pub fn validate(&mut self) -> Option<&str> {
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
        if !self.sort_by.is_empty() {
            self.generic_sort = true;
        }
        None
    }
}

#[derive(Debug)]
pub struct Col {
    pub attrs: Option<Vec<Attribute>>,
    pub name: Ident,
    pub element: P<Ty>,
    pub colty: P<Ty>,
}

#[derive(Debug, Copy, Clone)]
pub enum Serializer {
    Rustc,
    Serde,
}

