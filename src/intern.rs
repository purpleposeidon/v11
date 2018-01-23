//! This crate contains internal (but still public) items that are used by the macros.
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


/// Conversions for the results of `lock.(try_)?{read,write}`.
pub mod wrangle_lock {
    use std::sync::{LockResult, TryLockResult, TryLockError, PoisonError};

    pub fn map_result<G, F, R>(result: LockResult<G>, f: F) -> LockResult<R>
    where
        F: FnOnce(G) -> R
    {
        match result {
            Ok(t) => Ok(f(t)),
            Err(poison) => Err(PoisonError::new(f(poison.into_inner()))),
        }
    }

    pub fn map_try_result<G, F, R>(result: TryLockResult<G>, f: F) -> TryLockResult<R>
    where
        F: FnOnce(G) -> R
    {
        match result {
            Ok(t) => Ok(f(t)),
            Err(e) => Err(match e {
                TryLockError::Poisoned(poison) => TryLockError::Poisoned(PoisonError::new(f(poison.into_inner()))),
                TryLockError::WouldBlock => TryLockError::WouldBlock,
            }),
        }
    }
}
