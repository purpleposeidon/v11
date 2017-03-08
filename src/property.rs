// FIXME: The concept of domain belongs in a separate file.

use std::marker::PhantomData;
use std::any::Any;
use std::sync::RwLock;
use std::fmt;

use Universe;
use intern::PBox;
use intern;
use domain::*;

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub struct PropertyName(pub &'static str);
impl fmt::Display for PropertyName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub struct DomainedPropertyId(usize);
#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub struct GlobalPropertyId(pub usize);

/// Internal.
pub mod unset {
    use super::{DomainedPropertyId, GlobalPropertyId, DomainId};
    const UNSET: usize = ::std::usize::MAX;
    pub const DOMAIN_PROPERTY_ID: DomainedPropertyId = DomainedPropertyId(UNSET);
    pub const GLOBAL_PROPERTY_ID: GlobalPropertyId = GlobalPropertyId(UNSET);
    pub const DOMAIN_ID: DomainId = DomainId(UNSET);
}



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
    unsafe fn get(&self) -> &Prop<V>;
    fn register(&self);
}



/**
 * Usage example:
 * 
 * ```
 * # #[macro_use]
 * # extern crate v11;
 * mod table_use {}
 * domain! { EXAMPLE_DOMAIN }
 * property! { static EXAMPLE_DOMAIN/THING: i32 }
 * fn main() {
 *     EXAMPLE_DOMAIN.register_domain();
 *     THING.register();
 *     let universe = v11::Universe::new(&[EXAMPLE_DOMAIN]);
 *     {
 *         let mut val = universe[THING].write().unwrap();
 *         *val = 90;
 *     }
 *     assert_eq!(90, universe.get(THING));
 * }
 * ```
 *
 * Like `table!`, this macro requires access to a 'mod table_use'.
 * Like `constructor!`, this macro must be invoked from an externally visible module.
 * */
// FIXME: Better documentation.
#[macro_export]
macro_rules! property {
    (
        $(#[$ATTR:meta])*
        static $DOMAIN:ident/$NAME:ident: $TYPE:ty
    ) => {
        property! { static $DOMAIN/$NAME: $TYPE = Default::default(); }
    };
    (
        $(#[$ATTR:meta])*
        pub static $DOMAIN:ident/$NAME:ident: $TYPE:ty
    ) => {
        property! { pub static $DOMAIN/$NAME: $TYPE = Default::default(); }
    };
    (
        $(#[$ATTR:meta])*
        static $DOMAIN:ident/$NAME:ident: $TYPE:ty = $INIT:expr;
    ) => {
        $(#[$ATTR])*
        static $NAME: &'static $crate::property::ToPropRef<$TYPE> = &$NAME::PropRef as &$crate::property::ToPropRef<$TYPE>;

        #[doc(hidden)]
        #[allow(non_snake_case)]
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
        mod $NAME {
            property!(@mod $DOMAIN/$NAME: $TYPE = $INIT;);
        }
    };

    (@mod $DOMAIN:ident/$NAME:ident: $TYPE:ty = $INIT:expr;) => {
        #[allow(unused_imports)]
        use super::*;

        type Type = $TYPE;

        use $crate::intern::PBox;
        use $crate::property::{unset, PropertyName, Prop, ToPropRef, PropertyIndex};
        use $crate::domain::DomainName;
        use std::sync::RwLock;

        pub fn producer() -> PBox {
            let val: Type = $INIT;
            Box::new(RwLock::new(val)) as PBox
            // Yeah, this guy's a bit ridiculous.
        }

        // Can't access this directly because 'static mut' is unsafe to touch in any way.
        #[doc(hidden)]
        static mut VAL: Prop<Type> = Prop {
            domain_name: DomainName(stringify!($DOMAIN)),
            name: PropertyName(concat!(stringify!($DOMAIN), "/", stringify!($NAME))),
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
            // Unsafe as register could being called simultaneously.
            // Could have a lock thing to make things actually-safe, but, well...
            // That's overhead. Just do things in proper life-cycles.
            unsafe fn get(&self) -> &Prop<Type> {
                &VAL
            }

            fn register(&self) {
                let producer = producer as fn() -> PBox;
                unsafe {
                    VAL.init(producer);
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

    pub fn init(&mut self, producer: fn() -> PBox) {
        let mut pmap: &mut GlobalProperties = &mut *PROPERTIES.write().unwrap();
        // We must acquire the global lock at the beginning of this function. If we wait, and a
        // property is being registered from multiple threads simultaneous, there will be duplicate
        // registrations. This must happen before the if below.

        // There are 3 things that can happen:
        // 1: This function was called already on the same Prop.
        // 2: This is the first time a property with this name has been registered.
        // 3: Another PropRef with the same name
        if self.get_global_index() != unset::GLOBAL_PROPERTY_ID {
            // This handles the first case.
            return;
        }
        self.check_name();
        let mut first_instance = false;
        let mut domain_info = pmap.domains.get_mut(&self.domain_name).unwrap_or_else(|| panic!("Property {} is for an undefined domain", self));
        let _next_gid = GlobalPropertyId(pmap.gid2producer.len());
        let global_index = *pmap.name2gid.entry(self.name).or_insert_with(|| {
            first_instance = true;
            _next_gid
        });
        let domained_index = DomainedPropertyId(domain_info.property_members.len());
        self.index = PropertyIndex {
            domain_id: domain_info.id,
            domained_index: domained_index,
            global_index: global_index,
            v: PhantomData,
        };
        pmap.gid2name.insert(self.index.global_index, self.name);
        domain_info.property_members.push(global_index);
        if first_instance {
            pmap.gid2producer.push(producer);
        }
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


impl Universe {
    /// Returns a copy of the value of the given property. Only works for properties that are `Copy`.
    pub fn get<V: Any + Sync + Copy>(&self, prop: &ToPropRef<V>) -> V {
        use std::sync::RwLockReadGuard;
        let v: RwLockReadGuard<V> = self[prop].read().unwrap();
        *v
    }

    /// Sets the value of a property.
    pub fn set<V: Any + Sync>(&self, prop: &ToPropRef<V>, val: V) {
        *self[prop].write().unwrap() = val;
    }

    /// Adds any properties that are unknown. This function should be called if any libraries have
    /// been loaded since before the universe was created.
    pub fn add_properties(&mut self) {
        // We only allow domains to be set at creation, so we don't need to look for new ones.
        // Trying to get a property at a new domain is an errorneous/exceptional case, so this is
        // fine.
        let pmap = PROPERTIES.read().unwrap();
        for prop in &mut self.domains {
            if let MaybeDomain::Domain(ref mut instance) = *prop {
                instance.add_properties(&*pmap);
            }
        }
    }

    /// Return a list of the names of all registered domains.
    pub fn get_domain_names(&self) -> Vec<DomainName> {
        let mut ret = Vec::new();
        for domain in &self.domains {
            if let MaybeDomain::Domain(ref instance) = *domain {
                ret.push(instance.name);
            }
        }
        ret
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
                panic!("The property {:?} was not registered.", prop);
            },
            Some(&MaybeDomain::Unset(_))
            | None /* Must be some new fangled domain this Universe doesn't care about */
            => {
                panic!("The property {} is not in this Unvierse's domain.", prop);
            },
            Some(&MaybeDomain::Domain(ref e)) => e,
        };
        let l: Option<&RwLock<V>> = match domain_instance.property_members.get(prop.get_index_within_domain().0) {
            None => if prop.get_domain_id() == unset::DOMAIN_ID {
                panic!("The property {} was never initialized.", prop);
            } else {
                panic!("The universe does not know about property {}; perhaps there is a missed call to Universe.add_properties()?", prop);
            },
            Some(v) => {
                ::intern::desync_box(v).downcast_ref()
            },
        };
        match l {
            None => panic!("The type of property {} does not match the type of what the Universe has.", prop),
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
    enum BestColors {
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
        TEST.register_domain(); // FIXME: Not having to register the domain'd be nice. Can we avoid it?
        EXPLICIT_INIT.register();
        STANDARD_PROPERTY.register();
        PROP.register();
        MAP.register();
        BEST_COLORS.register();
        Universe::new(&[TEST])
    }
}
