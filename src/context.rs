//! `v11` code involving many table locks can encounter two problems.
//! 1. A higher function may have a write lock, and some lower function also needs a write locking,
//!    resulting in a dead-lock.
//! 2. Passing many locks around is unwieldy, but must be done in tight `for` loops.
//! You could create context structs manually, but this is labor-intensive, especially when you
//! start needing to combine them.
use std::os::raw::c_void;

#[doc(hidden)]
pub trait ReleaseFields {
    /// Swaps fields to another struct. `slot_for` is a function that returns a mutable
    /// pointer to the field with the type of the provided string, or `null_mut` if there is no
    /// such field. The field must be 'initialized' using `mem::zeroed()`. The second return value
    /// is the size of the field, and is used as a sanity-check.
    unsafe fn release_fields<F>(self, field_for: F)
    where F: FnMut(&'static str) -> (*mut c_void, usize);
}

/// Creates a struct that holds many table locks. This is useful for efficiently passing
/// whole lock contexts to other functions. It is possible to 'transfer' one context into another
/// using `NewContext::from(universe, oldContext)`. Any unused locks will be dropped, and any new
/// locks will be acquired.
/// 
/// The locks are duck-typed: any type with a function
/// `fn lock<'a>(&'a Universe) -> Self where Self: 'a`
/// can be used.
///
/// # Example
/// ```no_compile
/// context! {
///     mod my_context_throwaway_module_name;
///     pub struct MyContext {
///         reader: data_table::Read,
///         writer: data_log::Write,
///     }
/// }
/// ```
// This macro is Wildy Exciting.
#[macro_export]
macro_rules! context {
    (mod $nonce:ident; pub struct $name:ident {
        $($i:ident: $lock:path,)*
    }) => {
        #[doc(hidden)]
        mod $nonce {
            pub use std::mem;
            pub use std::ptr::null_mut;

            $(pub mod $i {
                // This funky business allows access to $lock as a type using $i::Lock.
                pub use $lock as Lock;
            })*
        }

        /// Holds locks for several tables.
        pub struct $name<'a> {
            $(pub $i: self::$nonce::$i::Lock<'a>,)*
        }
        impl<'a> $name<'a> {
            pub fn new(universe: &'a $crate::Universe) -> Self {
                use self::$nonce::*;
                Self {
                    $($i: $i::Lock::lock(universe),)*
                }
            }
        }

        impl<'a> $crate::context::ReleaseFields for $name<'a> {
            unsafe fn release_fields<F>(self, mut field_for: F)
            where F: FnMut(&'static str) -> (*mut ::std::os::raw::c_void, usize)
            {
                use self::$nonce::*;
                $({
                    let mut field = self.$i;
                    let (swap_to, size) = field_for($i::Lock::lock_name());
                    if swap_to.is_null() {
                        mem::drop(field);
                    } else {
                        let expect_size = mem::size_of::<$i::Lock>();
                        if size != expect_size {
                            panic!("sizes of {} did not match! {} vs {}", $i::Lock::lock_name(), size, expect_size);
                        }
                        // swap_to points at invalid memory
                        let swap_to = &mut *(swap_to as *mut $i::Lock);
                        mem::swap(&mut field, swap_to);
                        mem::forget(field);
                    }
                })*
                // Ring'll implement Drop, which would be a problem
                // if we didn't move all the fields out.
            }
        }

        impl<'a> $name<'a> {
            pub fn from<F>(universe: &'a $crate::Universe, old: F) -> Self
            where F: $crate::context::ReleaseFields
            {
                use self::$nonce::*;
                // We have a static list of our own fields, and we try to initialize them from
                // `old`'s. Since the macro doesn't actually know what fields `old` has, we need to
                // track which of our own fields we've initialized.
                // (FIXME: LLVM w/ --release should make this 0-cost; does it?)
                $(
                    let mut $i: (bool, $i::Lock<'a>);
                )*
                unsafe {
                    $(
                        $i = (false, mem::zeroed());
                    )*
                    old.release_fields(|name| {
                        $(if name == $i::Lock::lock_name() {
                            return if $i.0 {
                                // This case is likely a combined table. release_fields' contract
                                // requires dead memory, so this test is necessary.
                                (null_mut(), 0)
                            } else {
                                $i.0 = true;
                                (mem::transmute(&mut $i.1), mem::size_of::<$i::Lock>())
                            };
                        })*
                        (null_mut(), 0)
                    });
                    $(
                        if !$i.0 {
                            let mut new = $i::Lock::<'a>::lock(universe);
                            mem::swap(&mut new, &mut $i.1);
                            mem::forget(new);
                        }
                    )*
                }
                Self {
                    $($i: $i.1),*
                }
            }
        }
    };
}

/// Tuples of up to three contexts can be combined. Try nesting them if you want more.
pub mod merging_multiple_contexts {
    use super::*;

    impl ReleaseFields for () {
        unsafe fn release_fields<F>(self, _: F)
        where F: FnMut(&'static str) -> (*mut c_void, usize)
        {
        }
    }

    impl<A> ReleaseFields for (A,)
    where
        A: ReleaseFields,
    {
        unsafe fn release_fields<F>(self, field_for: F)
        where F: FnMut(&'static str) -> (*mut c_void, usize)
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
        where F: FnMut(&'static str) -> (*mut c_void, usize)
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
        where F: FnMut(&'static str) -> (*mut c_void, usize)
        {
            self.0.release_fields(|n| field_for(n));
            self.1.release_fields(|n| field_for(n));
            self.2.release_fields(|n| field_for(n));
        }
    }
}
