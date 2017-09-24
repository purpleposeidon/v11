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

/// Limits the lifetime of a reference.
pub struct RefA<'a, T: 'a>(&'a T);
/// Limits the lifetime of a mutable reference.
pub struct MutA<'a, T: 'a>(&'a mut T);
impl<'a, T: 'a> RefA<'a, T> {
    pub fn new(t: &'a T) -> Self {
        RefA(t)
    }
}
impl<'a, T: 'a> MutA<'a, T> {
    pub fn new(t: &'a mut T) -> Self {
        MutA(t)
    }
}
use std::ops::{Deref, DerefMut};
impl<'a, T: 'a> Deref for RefA<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0
    }
}
impl<'a, T: 'a> Deref for MutA<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0
    }
}
impl<'a, T: 'a> DerefMut for MutA<'a, T>  {
    fn deref_mut(&mut self) -> &mut T {
        self.0
    }
}
