use std::marker::PhantomData;
use std::any::Any;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::fmt;

use crate::Universe;
use crate::intern;
use crate::domain::*;

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
#[derive(Serialize, Deserialize)]
pub struct PropertyName(pub &'static str);
impl fmt::Display for PropertyName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[doc(hidden)]
#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub struct DomainedPropertyId(usize);
#[doc(hidden)]
#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub struct GlobalPropertyId(pub usize);

#[doc(hidden)]
pub mod unset {
    use super::{DomainedPropertyId, GlobalPropertyId, DomainId};
    const UNSET: usize = ::std::usize::MAX;
    pub const DOMAIN_PROPERTY_ID: DomainedPropertyId = DomainedPropertyId(UNSET);
    pub const GLOBAL_PROPERTY_ID: GlobalPropertyId = GlobalPropertyId(UNSET);
    pub const DOMAIN_ID: DomainId = DomainId(UNSET);
}



#[doc(hidden)]
#[derive(Copy, Clone, PartialEq)]
pub struct PropertyIndex<V> {
    pub domain_id: DomainId,
    pub domained_index: DomainedPropertyId,
    pub global_index: GlobalPropertyId,
    pub v: PhantomData<V>,
}
impl<V> fmt::Debug for PropertyIndex<V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "gid={} domain/domained={}/{}", self.global_index.0, self.domain_id.0, self.domained_index.0)
    }
}

pub trait ToPropRef<V: Sync>: Sync {
    fn name(&self) -> &'static str;
    unsafe fn get(&self) -> &Prop<V>;
    fn register(&self);
}



/**
 * Generates a property, which is a singleton value accessed via the [`Universe`].
 *
 * # Usage
 *
 * ```
 * # #[macro_use]
 * # extern crate v11;
 * domain! { EXAMPLE_DOMAIN }
 * property! { static EXAMPLE_DOMAIN/THING: i32 }
 * property! { static EXAMPLE_DOMAIN/NON_DEFAULT: i32 = 42; }
 * fn main() {
 *     EXAMPLE_DOMAIN.register();
 *     THING.register();
 *     let universe = v11::Universe::new(&[EXAMPLE_DOMAIN]);
 *     {
 *         let mut val = universe[THING].write().unwrap();
 *         *val = 90;
 *     }
 *     assert_eq!(90, universe.get(THING));
 * }
 * ```
 * */
// FIXME: Better documentation.
#[macro_export]
macro_rules! property {
    // Default-initialized properties
    (
        $(#[$ATTR:meta])*
        static $DOMAIN:ident/$NAME:ident: $TYPE:ty
    ) => {
        property! {
            $(#[$ATTR])*
            static $DOMAIN/$NAME: $TYPE = Default::default();
        }
    };
    (
        $(#[$ATTR:meta])*
        pub static $DOMAIN:ident/$NAME:ident: $TYPE:ty
    ) => {
        property! {
            $(#[$ATTR])*
            pub static $DOMAIN/$NAME: $TYPE = Default::default();
        }
    };

    // expression-initialized properties
    (
        $(#[$ATTR:meta])*
        static $DOMAIN:ident/$NAME:ident: $TYPE:ty = $INIT:expr;
    ) => {
        $(#[$ATTR])*
        static $NAME: &'static $crate::property::ToPropRef<$TYPE> = &$NAME::PropRef as &$crate::property::ToPropRef<$TYPE>;

        #[doc(hidden)]
        #[allow(non_snake_case)]
        #[allow(dead_code)]
        mod $NAME {
            property!(@mod $DOMAIN/$NAME: $TYPE = $INIT;);
        }
    };
    (
        $(#[$ATTR:meta])*
        pub static $DOMAIN:ident/$NAME:ident: $TYPE:ty = $INIT:expr;
    ) => {
        $(#[$ATTR])*
        pub static $NAME: &'static $crate::property::ToPropRef<$TYPE> = &$NAME::PropRef as &$crate::property::ToPropRef<$TYPE>;

        #[doc(hidden)]
        #[allow(non_snake_case)]
        #[allow(dead_code)]
        pub mod $NAME {
            property!(@mod $DOMAIN/$NAME: $TYPE = $INIT;);
        }
    };

    (@mod $DOMAIN:ident/$NAME:ident: $TYPE:ty = $INIT:expr;) => {
        #[allow(unused_imports)]
        use super::*;

        pub type Type = $TYPE;

        use $crate::intern::PBox;
        use $crate::property::{unset, PropertyName, Prop, ToPropRef, PropertyIndex};
        use $crate::domain::DomainName;
        use $crate::Universe;
        use $crate::context::Lockable;
        use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
        use std::ops::{Deref, DerefMut};

        pub fn read(universe: &Universe) -> RwLockReadGuard<Type> {
            universe[&PropRef].read().unwrap()
        }

        pub fn write(universe: &Universe) -> RwLockWriteGuard<Type> {
            universe[&PropRef].write().unwrap()
        }

        #[must_use]
        pub struct Read<'a>(RwLockReadGuard<'a, Type>);
        unsafe impl<'a> Lockable<'a> for Read<'a> {
            const TYPE_NAME: &'static str = concat!("ref v11/property/", stringify!($DOMAIN), "/", stringify!($NAME), ": ", stringify!($TYPE));
            fn lock(universe: &'a Universe) -> Self {
                Read(universe[&PropRef].read().unwrap())
            }
        }
        impl<'a> Deref for Read<'a> {
            type Target = Type;
            fn deref(&self) -> &Type {
                self.0.deref()
            }
        }

        #[must_use]
        pub struct Write<'a>(RwLockWriteGuard<'a, Type>);
        unsafe impl<'a> Lockable<'a> for Write<'a> {
            const TYPE_NAME: &'static str = concat!("mut v11/property/", stringify!($DOMAIN), "/", stringify!($NAME), ": ", stringify!($TYPE));
            fn lock(universe: &'a Universe) -> Self {
                Write(universe[&PropRef].write().unwrap())
            }
        }
        impl<'a> Deref for Write<'a> {
            type Target = Type;
            fn deref(&self) -> &Type {
                self.0.deref()
            }
        }
        impl<'a> DerefMut for Write<'a> {
            fn deref_mut(&mut self) -> &mut Type {
                self.0.deref_mut()
            }
        }

        struct Produce;
        impl $crate::domain::Producer for Produce {
            fn produce(&self) -> PBox {
                let val: Type = $INIT;
                Box::new(RwLock::new(val)) as PBox
            }
            fn domain(&self) -> DomainName { DOMAIN_NAME }
            fn name(&self) -> PropertyName { NAME }
        }

        const DOMAIN_NAME: DomainName = DomainName(stringify!($DOMAIN));
        const NAME: PropertyName = PropertyName(concat!(stringify!($DOMAIN), "/", stringify!($NAME)));

        // Can't access this directly because 'static mut' is unsafe to touch in any way.
        #[doc(hidden)]
        static mut VAL: Prop<Type> = Prop {
            domain_name: DOMAIN_NAME,
            name: NAME,
            index: PropertyIndex {
                domain_id: unset::DOMAIN_ID,
                global_index: unset::GLOBAL_PROPERTY_ID,
                domained_index: unset::DOMAIN_PROPERTY_ID,
                v: ::std::marker::PhantomData,
            },
        };

        #[derive(Clone, Copy)]
        #[doc(hidden)]
        pub struct PropRef;
        impl ToPropRef<Type> for PropRef {
            fn name(&self) -> &'static str { NAME.0 }
            // Unsafe as register could being called simultaneously.
            // Could have a lock thing to make things actually-safe, but, well...
            // That's overhead. Just do things in proper life-cycles.
            unsafe fn get(&self) -> &Prop<Type> {
                if VAL.index.domain_id == unset::DOMAIN_ID {
                    VAL.sync_alias();
                }
                &VAL
            }

            fn register(&self) {
                unsafe {
                    VAL.init(Box::new(Produce));
                }
            }
        }

    };
}

/**
 * Custom properties associated with a universe.
 * Access to a registered or previously used property is `O(1)`.
 * */
#[derive(Clone, Copy)]
pub struct Prop<V> {
    pub domain_name: DomainName,
    pub name: PropertyName,
    pub index: PropertyIndex<V>,
    // ideal: "universe[debug_enabled]"
    // Unfortunately it's more like "*universe[debug_enabled].read()", although
    // "universe.get(debug_enabled)" can be used when V is Copy.
}
impl<V> fmt::Debug for Prop<V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}[{:?}]", self.name.0, self.index)
    }
}
impl<V> Prop<V> {
    fn get_domain_id(&self) -> DomainId { self.index.domain_id }
    fn get_index_within_domain(&self) -> DomainedPropertyId { self.index.domained_index }
    fn get_global_index(&self) -> GlobalPropertyId { self.index.global_index }

    pub fn init(&mut self, producer: Box<crate::domain::Producer>) {
        let globals = clone_globals();
        let pmap: &mut GlobalProperties = &mut *globals.write().unwrap();
        // We must acquire the global lock at the beginning of this function. If we wait, and a
        // property is being registered from multiple threads simultaneously, there will be duplicate
        // registrations. This must happen before the if below.

        // These are the states:
        // 1: This function was called already on the same Prop.
        // 2: This is the first time a property with this name has been registered.
        // 3: A twin PropRef was already registered.
        if self.get_global_index() != unset::GLOBAL_PROPERTY_ID {
            // This handles the first case.
            return;
        }
        self.check_name();
        let mut first_instance = false;
        let domain_info = pmap.domains.get_mut(&self.domain_name).unwrap_or_else(|| panic!("Property {} is for an undefined domain", self));
        if crate::domain::check_lock() && domain_info.locked() {
            panic!("Adding {:?} on a locked domain", self);
        }
        let global_index = {
            let next_id = GlobalPropertyId(pmap.gid2producer.len());
            *pmap.name2gid.entry(self.name).or_insert_with(|| {
                first_instance = true;
                next_id
            })
        };
        let domained_index = {
            let next_id = DomainedPropertyId(domain_info.property_members.len());
            *domain_info.name2did.entry(self.name).or_insert(next_id)
        };
        self.index = PropertyIndex {
            domain_id: domain_info.id,
            domained_index: domained_index,
            global_index: global_index,
            v: PhantomData,
        };
        pmap.gid2name.insert(self.index.global_index, self.name);
        domain_info.property_members.push(global_index);
        if first_instance {
            pmap.gid2producer.push(FmtProducer(producer));
        }
        // FIXME: Shouldn't we panic if we're adding something to a domain that was already used to
        // make a universe?
    }

    #[cold]
    pub fn sync_alias(&mut self) {
        // 4: A twin PropRef was already registered, but we weren't. To keep things easy, we'll
        //    just silently fix ourselves.
        // We don't need any sanity checks here. If a twin is registered, then they're already
        // sane. If not, then we panic.
        let globals = clone_globals();
        let pmap: &mut GlobalProperties = &mut *globals.write().unwrap();
        let domain_info = pmap.domains.get_mut(&self.domain_name).unwrap_or_else(|| panic!("Property {} is for an undefined domain", self));
        let global_index = *pmap.name2gid.get(&self.name).unwrap_or_else(|| panic!("Property {:?} was never registered", self));
        let domained_index = *domain_info.name2did.get(&self.name).expect("gid & did both registered" /* name2gid panic logically occludes/equals this */);
        self.index = PropertyIndex {
            domain_id: domain_info.id,
            domained_index: domained_index,
            global_index: global_index,
            v: PhantomData,
        };
    }

    fn check_name(&self) {
        intern::check_name(self.domain_name.0);
        let mut parts = self.name.0.splitn(2, '/');
        let domain_name = parts.next().expect("str::split failed?");
        let name = parts.next().unwrap_or_else(|| panic!("{:?} is not in the format 'domain/name'", self.name.0));
        if domain_name != self.domain_name.0 {
            panic!("Domain names do not match: {:?}, {:?}", domain_name, self.domain_name);
        }
        intern::check_name(name);
    }
}
impl<V> fmt::Display for Prop<V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}


/// Property manipulation methods.
impl Universe {
    /// Returns a copy of the value of the given property. Only works for properties that are `Copy`.
    pub fn get<V: Any + Sync + Copy>(&self, prop: &ToPropRef<V>) -> V {
        let v: RwLockReadGuard<V> = self[prop].read().unwrap();
        *v
    }

    /// Sets the value of a property.
    pub fn set<V: Any + Sync>(&self, prop: &ToPropRef<V>, val: V) {
        *self[prop].write().unwrap() = val;
    }

    /// Exchange the value in a property.
    pub fn swap<V: Any + Sync>(&self, prop: &ToPropRef<V>, mut val: V) -> V {
        let mut prop = self[prop].write().unwrap();
        ::std::mem::swap(&mut *prop, &mut val);
        val
    }

    /// Gets the property locked for reading. Panics if poisoned.
    pub fn read<V: Any + Sync>(&self, prop: &ToPropRef<V>) -> RwLockReadGuard<V> {
        self[prop].read().unwrap()
    }

    /// Gets the property locked for writing. Panics if poisoned.
    pub fn write<V: Any + Sync>(&self, prop: &ToPropRef<V>) -> RwLockWriteGuard<V> {
        self[prop].write().unwrap()
    }
}
impl<'a, V: Any + Sync> ::std::ops::Index<&'a ToPropRef<V>> for Universe {
    type Output = RwLock<V>;
    fn index(&self, prop: &'a ToPropRef<V>) -> &RwLock<V> {
        let prop: &Prop<V> = unsafe {
            // FIXME: Just don't call Prop.register() at the same time as this!
            prop.get()
        };
        let domain = self.domains.get(prop.get_domain_id().0);
        let domain_instance: &DomainInstance = match domain {
            None if prop.get_domain_id() == unset::DOMAIN_ID => {
                panic!("The property {:?} was not registered.", prop)
            },
            Some(&MaybeDomain::Unset(_))
            | None /* Must be some new fangled domain this Universe doesn't care about */
            => {
                panic!("The property {} is not in this Universe's domain.", prop)
            },
            Some(&MaybeDomain::Domain(ref e)) => e,
        };
        let domained_index = prop.get_index_within_domain().0;
        let v = match domain_instance.property_members.get(domained_index) {
            None => if prop.get_domain_id() == unset::DOMAIN_ID {
                panic!("The property {} was never initialized.", prop)
            } else {
                panic!("The property {} was added to the domain AFTER this Universe was created.", prop)
            },
            Some(v) => v,
        };
        let l: Option<&RwLock<V>> = v.downcast_ref();
        match l {
            None => {
                panic!("Downcast of property {} failed.", prop)
            },
            // FIXME: Say what the type is?
            Some(ret) => ret
        }
    }
}

#[cfg(test)]
pub /* property! requires this */ mod test {
    use super::super::*;

    domain! { TEST }

    #[test]
    fn void_universe1() {
        test_universe();
        Universe::new(&[]);
    }

    #[test]
    fn void_universe2() {
        Universe::new(&[]);
    }

    property! { static TEST/EXPLICIT_INIT: usize }
    #[test]
    fn explicit_init() {
        let universe = test_universe();
        assert_eq!(0, universe.get(EXPLICIT_INIT));
    }

    property! { static TEST/STANDARD_PROPERTY: i64 }
    #[test]
    fn standard_property_usage() {
        let universe = test_universe();
        assert_eq!(0, universe.get(STANDARD_PROPERTY));
        universe.set(STANDARD_PROPERTY, 20);
        assert_eq!(20, universe.get(STANDARD_PROPERTY));
    }

    property! { static TEST/PROP: usize }
    #[test]
    fn property_basics() {
        let universe = test_universe();

        // default value
        assert!(universe.get(PROP) == 0);

        // set should work
        universe.set(PROP, 99);
        assert!(universe.get(PROP) == 99);
    }

    property! { static TEST/MAP: ::std::collections::HashMap<String, i32> }

    #[test]
    fn hashmaps_with_threads() {
        let universe = test_universe().guard();
        {
            let universe = universe.read().unwrap();
            let mut map = universe[MAP].write().unwrap();
            map.insert("foo".to_string(), 90);
        }
        {
            let universe = universe.read().unwrap();
            let map = universe[MAP].read().unwrap();
            println!("{}", map.get("foo").unwrap());
        }
        let thread = {
            let universe = universe.clone();
            ::std::thread::spawn(move || {
                let universe = universe.read().unwrap();
                //println!("{:?}", &*universe);
                let map = universe[MAP].read().unwrap();
                let got = map.get("foo").unwrap();
                assert_eq!(got, &90);
            })
        };
        let j = thread.join();
        {
            let universe = universe.read().unwrap();
            let map = universe[MAP].read().unwrap();
            println!("After join: {}", map.get("foo").unwrap());
        }
        j.expect("thread failed");
        {
            let universe = universe.read().unwrap();
            let map = universe[MAP].read().unwrap();
            println!("main says: {}", map.get("foo").unwrap());
        }
    }

    #[test]
    fn hashmaps_many_times() {
        for _ in 0..100 {
            hashmaps_with_threads();
        }
    }

    #[derive(PartialEq, Eq, Clone, Copy, Debug)]
    pub enum BestColors {
        #[allow(dead_code)]
        Purple,
        Pink,
    }

    property! { static TEST/BEST_COLORS: BestColors = BestColors::Pink; }

    #[test]
    fn very_best_color() {
        let universe = test_universe();
        assert_eq!(universe.get(BEST_COLORS), BestColors::Pink);
    }

    fn test_universe() -> Universe {
        TEST.register(); // FIXME: Not having to register the domain'd be nice. Can we avoid it?
        EXPLICIT_INIT.register();
        STANDARD_PROPERTY.register();
        PROP.register();
        MAP.register();
        BEST_COLORS.register();
        Universe::new(&[TEST])
    }

    property! { static TEST/LATE: usize }

    #[test]
    #[should_panic(expected = "added to the domain AFTER this Universe was created")]
    fn domain_locking() {
        // This test is poisonous!
        // So long as this is the only one, we can cheat! But don't have a slow computer!
        // (Otherwise we could use #[ignore].)
        ::std::thread::sleep(::std::time::Duration::from_millis(200));

        let verse = test_universe();
        LATE.register();
        let _ = verse[LATE].read().unwrap();
    }

    mod alias {
        /// Primary
        pub mod foo {
            domain! { pub ALIAS }
            property! { pub static ALIAS/BLAH: i32 }
        }
        /// Implicitly registered secondary
        pub mod bar {
            domain! { pub ALIAS }
            property! { pub static ALIAS/BLAH: i32 }
        }
        /// Explicitly registered secondary
        pub mod baz {
            domain! { pub ALIAS }
            property! { pub static ALIAS/BLAH: i32 }
        }

        #[test]
        fn test_aliased() {
            use crate::Universe;
            foo::ALIAS.register();
            foo::BLAH.register();
            baz::ALIAS.register();
            baz::BLAH.register();
            #[allow(unused_variables)]
            {
                // All should be constructable
                let verse = Universe::new(&[foo::ALIAS]);
                let verse = Universe::new(&[bar::ALIAS]);
                let verse = Universe::new(&[baz::ALIAS]);
            }
            let verse = Universe::new(&[foo::ALIAS]);
            *verse[foo::BLAH].write().unwrap() = 10;
            assert_eq!(*verse[baz::BLAH].read().unwrap(), 10);
            assert_eq!(*verse[bar::BLAH].read().unwrap(), 10);
        }
    }
}
