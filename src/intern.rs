//! This crate contains internal (but still public) items that are used by the macros.
//! User-code should not use this directly.

use std::ops::{Deref, DerefMut};

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

/// Two different types that dereferenced into the same type.
pub enum BiRef<A, B, T>
where
    A: Deref<Target = T>,
    B: Deref<Target = T>,
{
    Left(A),
    Right(B),
}
impl<A, B, T> Deref for BiRef<A, B, T>
where
    A: Deref<Target = T>,
    B: Deref<Target = T>,
{
    type Target = T;
    fn deref(&self) -> &T {
        match self {
            &BiRef::Left(ref a) => a.deref(),
            &BiRef::Right(ref b) => b.deref(),
        }
    }
}

pub struct GenerativeIter<F>(pub F);
impl<F, R> Iterator for GenerativeIter<F>
where F: FnMut() -> Option<R>
{
    type Item = R;
    fn next(&mut self) -> Option<R> {
        self.0()
    }
}

pub enum MaybeBorrow<'a, T: 'a> {
    Owned(T),
    Nothing,
    Borrow(&'a mut T),
}
impl<'a, T: 'a> MaybeBorrow<'a, T> {
    pub fn is_owned(&self) -> bool {
        // 🌽🌽🌽🌽🌽🌽🌽
        match self {
            MaybeBorrow::Owned(_) => true,
            _ => false,
        }
    }
    pub fn is_missing(&self) -> bool {
        match self {
            MaybeBorrow::Nothing => true,
            _ => false,
        }
    }
}
impl<'a, T: 'a> Deref for MaybeBorrow<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        match self {
            MaybeBorrow::Owned(ref t) => t,
            MaybeBorrow::Nothing => panic!("neither borrowed nor owned"),
            MaybeBorrow::Borrow(t) => t,
        }
    }
}
impl<'a, T: 'a> DerefMut for MaybeBorrow<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        match self {
            MaybeBorrow::Owned(ref mut t) => t,
            MaybeBorrow::Nothing => panic!("neither borrowed nor owned"),
            MaybeBorrow::Borrow(t) => t,
        }
    }
}
