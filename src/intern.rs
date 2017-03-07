//! This crate contains internal (but still public) items that are used by the `table!` and
//! `property!` macros.
//! User-code should not use this directly.

pub fn check_name(name: &str) {
    match name.chars().next() {
        None => panic!("Empty name"),
        Some('_') => panic!("Reserved name {}", name),
        Some(c) if (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z') => (),
        _ => panic!("Invalid name {:?}", name),
    }
    for c in name.chars() {
        if c == '_' { continue; }
        if c >= 'A' && c <= 'Z' { continue; }
        if c >= 'a' && c <= 'z' { continue; }
        if c >= '0' && c <= '9' { continue; }
        panic!("Invalid name {:?}", name);
    }
}

// FIXME: mopa?
use std::any::Any;
pub type PBox = Box<Any + Send + Sync>;


pub struct VoidIter<I>(I);
impl<I> Iterator for VoidIter<I> {
    type Item = I;
    fn next(&mut self) -> Option<I> { None }
}


// FIXME: Are these actually necessary?
pub fn desync_box<'a>(v: &'a PBox) -> &'a Any {
    use std::ops::Deref;
    v.deref()
}

pub fn desync_box_mut<'a>(v: &'a mut PBox) -> &'a mut Any {
    use std::ops::DerefMut;
    v.deref_mut()
}

