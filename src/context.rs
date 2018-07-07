//! Ergonomics for juggling multiple locks.
//!
//! `v11` code involving many table locks can encounter two problems:
//!
//! 1. A higher function may have a write lock, and some lower function also needs a write locking,
//!    resulting in a dead-lock.
//! 2. Passing many locks around is unwieldy, but must be done within tight `for` loops.
//!
//! You could create context structs manually, but this is labor-intensive, especially when you
//! start needing to combine them.
//!
//! This module introduces [`context!`] to help with this.
use std::any::Any;
use Universe;


/// A struct that can be moved piece-wise.
pub trait ReleaseFields {
    /// Moves this struct's contents into another one,
    /// passing each field through the function parameter.
    /// The function requires the name of the field,
    /// and the field's value in a `&mut Option<F>` cast to `&mut Any`.
    fn release_fields<F>(self, give: F)
    where F: FnMut(&'static str, &mut Any);
}

/// This trait indicates a type that can be locked by `context!`.
pub trait Lockable<'u> {
    /// A unique name for the type.
    const TYPE_NAME: &'static str;

    /// Create the lock.
    fn lock(&'u Universe) -> Self where Self: 'u;
}

/// Creates a struct that holds many table locks that implement [`Lockable`].
/// This is useful for ergonomically passing multiple locks to other functions.
/// It is possible to 'transfer' the one context's locks into another using `NewContext::from(universe, oldContext)`.
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
#[macro_export]
macro_rules! context {
    (pub struct $name:ident {
        $(pub $i:ident: $lock:path,)*
    }) => {
        // Shame there isn't some kind of identifier concatenation macro.
        // I tried using `mod $name`, but forget why that doesn't work.
        #[doc(hidden)]
        pub mod __hi_there__please_put_each_context_in_its_own_module {
            use std::any::Any;
            use $crate::context::Lockable;
            use $crate::Universe;

            $(mod $i {
                #[allow(unused)]
                use super::super::*;
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
                pub fn new(universe: &'a Universe) -> Self {
                    Self {
                        $($i: $i::Lock::lock(universe),)*
                    }
                }
            }

            impl<'a> $crate::context::ReleaseFields for $name<'a> {
                fn release_fields<F>(self, mut give: F)
                where F: FnMut(&'static str, &mut Any)
                {
                    $({
                        const NAME: &'static str = <self::$i::Lock as Lockable>::TYPE_NAME;
                        let mut field = Some(self.$i);
                        give(NAME, unsafe { ::std::mem::transmute(&mut field as &mut Any) });
                    })*
                }
            }

            impl<'a> $name<'a> {
                /// Create a context from another one, recycling any locks that are in both, and
                /// dropping any that are not.
                pub fn from<F>(universe: &'a Universe, old: F) -> Self
                where F: $crate::context::ReleaseFields
                {
                    // We have a static list of our own fields, and we try to initialize them from
                    // `old`'s. Since the macro doesn't actually know what fields `old` has, we need to
                    // track which of our own fields we've initialized.
                    // (FIXME: LLVM w/ --release should make this 0-cost; does it?)
                    $(
                        let mut $i: Option<$i::Lock<'a>> = None;
                    )*
                    old.release_fields(|name, field| match name {
                        $(<self::$i::Lock as Lockable>::TYPE_NAME => {
                            if $i.is_some() {
                                // Identical read lock from combined contexts.
                                return;
                            }
                            if Some(field) = field.downcast_mut::<Option<$i::Lock<'_>>>() {
                                $i = field.take();
                            } else {
                                // Odd!
                                // 1. Two different types have the same name.
                                // 2. Two types from different compiler versions?
                                // We're just slightly less efficient is all.
                            }
                        },)*
                        _ => (),
                    });
                    Self {
                        $($i: if let Some(v) = $i {
                            v
                        } else {
                            <self::$i::Lock as Lockable>::lock(universe)
                        },)*
                    }
                }
            }
        }
        pub use self::__hi_there__please_put_each_context_in_its_own_module::$name;
    };
}

mod merging_multiple_contexts {
    use super::*;

    impl ReleaseFields for () {
        fn release_fields<F>(self, _give: F)
        where F: FnMut(&'static str, &mut Any)
        {
        }
    }

    impl<A> ReleaseFields for (A,)
    where
        A: ReleaseFields,
    {
        fn release_fields<F>(self, give: F)
        where F: FnMut(&'static str, &mut Any)
        {
            self.0.release_fields(give);
        }
    }

    impl<A, B> ReleaseFields for (A, B)
    where
        A: ReleaseFields,
        B: ReleaseFields,
    {
        fn release_fields<F>(self, mut give: F)
        where F: FnMut(&'static str, &mut Any)
        {
            self.0.release_fields(|n, f| give(n, f));
            self.1.release_fields(|n, f| give(n, f));
        }
    }

    impl<A, B, C> ReleaseFields for (A, B, C)
    where
        A: ReleaseFields,
        B: ReleaseFields,
        C: ReleaseFields,
    {
        fn release_fields<F>(self, mut give: F)
        where F: FnMut(&'static str, &mut Any)
        {
            self.0.release_fields(|n, f| give(n, f));
            self.1.release_fields(|n, f| give(n, f));
            self.2.release_fields(|n, f| give(n, f));
        }
    }
}
