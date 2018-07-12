//! Ergonomics for juggling multiple locks.
//!
//! `v11` code involving many table locks can encounter two problems:
//!
//! 1. A higher function may have a write lock, and some lower function also needs a write locking,
//!    resulting in a dead-lock.
//! 2. Passing many locks around is unwieldy, but must be done in tight `for` loops.
//!
//! You could create context structs manually, but this is labor-intensive, especially when you
//! start needing to combine them.
//!
//! This module introduces [`context!`] to help with this.
use std::os::raw::c_void;
use std::any::TypeId;
use Universe; // This could be parameterized to make this module v11-agnostic!


#[doc(hidden)]
pub trait ReleaseFields {
    /// Swaps fields to another struct. `slot_for` is a function that returns a mutable
    /// pointer to the field with the type of the provided string, or `null_mut` if there is no
    /// such field. The field must be 'initialized' using `mem::zeroed()`. The second return value
    /// is the size of the field, and is used as a sanity-check.
    ///
    /// The third return value is type `TypeId` of `PhantomData<T>`, or of `()` for absent fields.
    unsafe fn release_fields<F>(self, field_for: F)
    where F: FnMut(&'static str) -> (*mut c_void, usize, TypeId);
}

/// This trait indicates a type that can be locked by `context!`.
pub unsafe trait Lockable<'u> {
    /// It is very important that this name be unique per-type!
    /// It is relied upon to be unique per-type.
    const TYPE_NAME: &'static str;
    fn lock(&'u Universe) -> Self where Self: 'u;
    // What if we got a vtable? Each type should have a unique one...
}

/// Creates a struct that holds many table locks that implement `Lockable`.
/// This is useful for ergonomically passing multiple locks to other functions.
/// It is possible to 'transfer' one context into another using `NewContext::from(universe, oldContext)`.
/// Any unused locks will be dropped, and any new locks will be acquired.
///
/// Tuples of up to three contexts can be combined. Try nesting the tuples if you need more.
///
/// This macro can't be invoked more than once in the same module; you can invoke it in a
/// sub-module if necessary.
///
/// # Example
/// ```no_compile
/// context! {
///     pub struct MyContext {
///         pub reader: data_table::Read,
///         pub writer: data_log::Write,
///     }
/// }
/// ```
///
/// You might consider implementing convenience functions on the context struct.
// This macro is Wildly Exciting.
#[macro_export]
macro_rules! context {
    (pub struct $name:ident {
        $(pub $i:ident: $lock:path,)*
    }) => {
        // It's a shame there isn't some kind of identifier concatenation macro.
        #[allow(non_snake_case)]
        pub mod context_module {
            use std::mem;
            use std::ptr::null_mut;
            use std::marker::PhantomData;
            use std::any::TypeId;
            use $crate::context::Lockable;

            $(mod $i {
                #[allow(unused)]
                use super::super::*; // super::duper::*
                // This funky business allows access to `$lock` as a type using `self::$i::Lock`,
                // which is required due to macro restrictions.
                pub use $lock as Lock;
            })*

            /// Holds locks for any number of tables or properties.
            pub struct $name<'a> {
                $(pub $i: self::$i::Lock<'a>,)*
            }
            impl<'a> $name<'a> {
                /// Create a fresh context.
                pub fn new(universe: &'a $crate::Universe) -> Self {
                    Self {
                        $($i: $i::Lock::lock(universe),)*
                    }
                }
            }

            #[allow(unused)]
            pub fn new(universe: &$crate::Universe) -> $name {
                $name::new(universe)
            }

            impl<'a> $crate::context::ReleaseFields for $name<'a> {
                unsafe fn release_fields<F>(self, mut field_for: F)
                where F: FnMut(&'static str) -> (*mut ::std::os::raw::c_void, usize, TypeId)
                {
                    // Why c_void? Why not... T?
                    // Because T can't be Any, because Any requires 'static,
                    // and these are lock guards.
                    $({
                        let mut field = self.$i;
                        const TYPE_NAME: &'static str = <self::$i::Lock as Lockable>::TYPE_NAME;
                        let (swap_to, size, id) = field_for(TYPE_NAME);
                        if swap_to.is_null() {
                            mem::drop(field);
                        } else {
                            let expect_id = TypeId::of::<PhantomData<$i::Lock>>();
                            if expect_id != id {
                                // FIXME: Investigate relying only on TypeId. Dylibs?
                                panic!("TypeId of {} did not match!",
                                       <self::$i::Lock as Lockable>::TYPE_NAME);
                            }
                            let expect_size = mem::size_of::<$i::Lock>();
                            if size != expect_size {
                                panic!("sizes of {} did not match! {} vs {}",
                                       <self::$i::Lock as Lockable>::TYPE_NAME, size, expect_size);
                            }
                            // swap_to points at invalid memory
                            let swap_to = &mut *(swap_to as *mut $i::Lock);
                            mem::swap(&mut field, swap_to);
                            mem::forget(field);
                        }
                    })*
                    // $name'll implement Drop, which would be a problem
                    // if we didn't move all the fields out.
                }
            }

            impl<'a> $name<'a> {
                /// Create a context from another one, recycling any locks that are in both, and
                /// dropping any that are not.
                pub fn from<F>(universe: &'a $crate::Universe, old: F) -> Self
                where F: $crate::context::ReleaseFields
                {
                    // We have a static list of our own fields, and we try to initialize them from
                    // `old`'s. Since the macro doesn't actually know what fields `old` has, we need to
                    // track which of our own fields we've initialized.
                    // (FIXME: LLVM w/ --release should make this 0-cost; does it?)
                    $(
                        let mut $i: Option<$i::Lock<'a>> = None;
                    )*
                    unsafe {
                        old.release_fields(|name| {
                            match name {
                                $(<self::$i::Lock as Lockable>::TYPE_NAME => {
                                    if $i.is_some() {
                                        // This case is likely a combined table. release_fields' contract
                                        // requires dead memory, so this test is necessary.
                                        (null_mut(), 0, TypeId::of::<()>())
                                    } else {
                                        $i = Some(mem::zeroed());
                                        let ptr = $i.as_mut().unwrap();
                                        (
                                            mem::transmute(ptr),
                                            mem::size_of::<$i::Lock>(),
                                            TypeId::of::<PhantomData<$i::Lock>>(),
                                        )
                                    }
                                },)*
                                _ => (null_mut(), 0, TypeId::of::<()>()),
                            }
                        });
                        $(
                            if $i.is_none() {
                                let l = <self::$i::Lock as Lockable>::lock(universe);
                                $i = Some(l);
                            }
                        )*
                    }
                    Self {
                        $(
                            $i: $i.unwrap(),
                        )*
                    }
                }
            }

            #[allow(unused)]
            pub fn from<F>(universe: &$crate::Universe, old: F) -> $name
            where F: $crate::context::ReleaseFields
            {
                $name::from(universe, old)
            }
        }
        pub use self::context_module::*;
    };
}

mod merging_multiple_contexts {
    use super::*;

    impl ReleaseFields for () {
        unsafe fn release_fields<F>(self, _: F)
        where F: FnMut(&'static str) -> (*mut c_void, usize, TypeId)
        {
        }
    }

    impl<A> ReleaseFields for (A,)
    where
        A: ReleaseFields,
    {
        unsafe fn release_fields<F>(self, field_for: F)
        where F: FnMut(&'static str) -> (*mut c_void, usize, TypeId)
        {
            self.0.release_fields(field_for);
        }
    }

    impl<A, B> ReleaseFields for (A, B)
    where
        A: ReleaseFields,
        B: ReleaseFields,
    {
        unsafe fn release_fields<F>(self, mut field_for: F)
        where F: FnMut(&'static str) -> (*mut c_void, usize, TypeId)
        {
            self.0.release_fields(|n| field_for(n));
            self.1.release_fields(|n| field_for(n));
        }
    }

    impl<A, B, C> ReleaseFields for (A, B, C)
    where
        A: ReleaseFields,
        B: ReleaseFields,
        C: ReleaseFields,
    {
        unsafe fn release_fields<F>(self, mut field_for: F)
        where F: FnMut(&'static str) -> (*mut c_void, usize, TypeId)
        {
            self.0.release_fields(|n| field_for(n));
            self.1.release_fields(|n| field_for(n));
            self.2.release_fields(|n| field_for(n));
        }
    }
}
