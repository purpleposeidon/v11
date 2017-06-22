use syntex_syntax::ast::{Ident, Ty, Attribute};
use syntex_syntax::ptr::P;

#[derive(Debug)]
#[derive(Default)] // Don't actually use this, it's just to keep new() easy.
pub struct Table {
    // Header
    pub attrs: Vec<Attribute>,
    pub is_pub: bool,
    pub domain: String,
    pub name: String,
    pub cols: Vec<Col>,

    // Modifiers
    pub debug: bool,
    pub version: usize,
    pub row_id: String,
    pub sync_rm: Option<String>,
    pub free_list: bool,
    pub save: bool,
    pub track_changes: bool,
    pub generic_sort: bool,
    pub sort_by: Vec<String>,
    pub static_data: bool,
    pub no_complex_mut: bool,

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
    pub(crate) fn validate(&mut self, token: &super::ConstTokens) -> Option<&'static str> {
        if self.domain.is_empty() {
            return Some("No domain");
        }
        if self.name.is_empty() {
            return Some("No name");
        }
        if self.cols.is_empty() {
            return Some("No columns");
        }
        if self.static_data {
            if self.track_changes
                || self.sync_rm.is_some()
                || self.free_list
                || !self.track_changes
                || !self.sort_by.is_empty() {
                return Some("static tables shouldn't have modification features");
            }
        }
        if !self.sort_by.is_empty() {
            self.generic_sort = true;
        }
        if self.track_changes {
            if self.generic_sort || !self.sort_by.is_empty() {
                return Some("Change tracking is incompatible with sorting.");
            }
            self.no_complex_mut = true;
            self.cols.push(Col {
                attrs: None,
                name: token._event_name.clone(),
                element: token._event_element.clone(),
                colty: token._event_colty.clone(),
            });
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
