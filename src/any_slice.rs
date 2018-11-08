//! `&[T]`s can't be converted to `&Any`, because it wants to cast `[T]`, which is impossible
//! because `[T]` is unsized. And casting `&&[T]` is also impossible because `Any: 'static`.
//!
//! Do not confuse this type for `&[&Any]` or `&[Any]`.

use std::marker::PhantomData;
use std::any::{TypeId, Any};
use std::slice;

/// A homogeneous slice of uncertain type.
#[derive(Copy, Clone, Debug)]
pub struct AnySliceRef<'a> {
    _lifetime: &'a PhantomData<()>,
    id: TypeId,
    ptr: *const (),
    len: usize,
}
impl<'a> AnySliceRef<'a> {
    // `'a` in `&'a [E]` prevents lifetime leak.
    // Making AnySliceRef to be Sync/Send would require them, yeah?
    // However I don't need to think about it ATM.
    pub fn from<E>(slice: &'a [E]) -> Self
    where E: Any + Send + Sync
    {
        AnySliceRef {
            _lifetime: &PhantomData,
            id: TypeId::of::<E>(),
            ptr: slice.as_ptr() as *const (),
            len: slice.len(),
        }
    }

    pub fn downcast<E>(&'a self) -> Option<&'a [E]>
    where E: Any + Send + Sync
    {
        if self.id != TypeId::of::<E>() { return None; }
        Some(unsafe {
            // The soundness of this is dead-obvious.
            slice::from_raw_parts(
                self.ptr as *const E,
                self.len,
            )
        })
    }

    pub fn is_empty(&self) -> bool { self.len == 0 }
    pub fn element_typeid(&self) -> TypeId { self.id }
    pub fn len(&self) -> usize { self.len }
}
