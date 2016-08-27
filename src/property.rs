
use std::marker::PhantomData;
use std::collections::HashMap;
use std::any::Any;
use std::sync::RwLock;

use super::Universe;
use super::intern::PBox;


type FatPtr = (usize, usize);

struct GlobalProperties {
    name2id: HashMap<String, usize>,

    //id2producer: Vec<FatPtr>,
    // What that is really a Vec of:
    id2producer: Vec<fn() -> PBox>,
}

lazy_static! {
    static ref PROPERTIES: RwLock<GlobalProperties> = RwLock::new(GlobalProperties {
        name2id: HashMap::new(),
        id2producer: Vec::new(),
    });
}

#[derive(Copy, Clone, PartialEq)]
pub struct PropertyIndex<V> {
    pub i: usize,
    pub v: PhantomData<V>,
}

pub trait ToPropRef<V: Default + Sync>: Sync {
    fn get(&self) -> &'static Prop<V>;
}


use std::usize;
pub const UNSET: usize = usize::MAX;

/**
 * Usage example:
 * 
 * property!{static THING: i32}
 * # fn main() {
 * let universe = v11::Universe::new();
 * let mut val = universe[THING].write().unwrap();
 * *val = 90;
 * # }
 *
 * Like table!, this macro requires access to a 'mod table_use'.
 * */
// FIXME: Better documentation.
#[macro_export]
macro_rules! property {
    // There's room for improvement here. :/
    ($(#[$ATTR:meta])+ static $NAME:ident: $TYPE:ty) => {
        property!($(#[$ATTR])* static $NAME (stringify!($NAME)): $TYPE);
    };
    (static $NAME:ident: $TYPE:ty) => {
        property!(static $NAME (stringify!($NAME)): $TYPE);
    };
    ($(#[$ATTR:meta])+ static $NAME:ident ($NAMESTR:expr): $TYPE:ty) => {
        property!($NAME ($NAMESTR): $TYPE $(; #[$ATTR])*);
    };
    (static $NAME:ident ($NAME_STR:expr): $TYPE:ty $(; #[$ATTR:meta])*) => {
        #[allow(non_snake_case)]
        pub mod $NAME {
            use $crate::intern::PBox;
            use $crate::property::*;

            #[allow(unused_imports)]
            use super::table_use::*;
            
            constructor! { init }
            #[allow(dead_code)]
            extern fn init() {
                unsafe {
                    VAL.init(producer as fn() -> PBox)
                };
            }

            use std::sync::RwLock;
            fn producer() -> PBox {
                type TheType = $TYPE;
                Box::new(RwLock::new(TheType::default())) as PBox
            }

            // Can't access this directly because 'static mut' is unsafe to touch in any way.
            static mut VAL: Prop<$TYPE> = Prop {
                name: $NAME_STR,
                index: PropertyIndex {
                    i: UNSET,
                    v: ::std::marker::PhantomData,
                },
            };

            #[derive(Clone, Copy)]
            pub struct PropRef;
            impl ToPropRef<$TYPE> for PropRef {
                fn get(&self) -> &'static Prop<$TYPE> {
                    unsafe { &VAL }
                }
            }
        }

        $(#[$ATTR])*
        pub static $NAME: &'static $crate::property::ToPropRef<$TYPE> = &$NAME::PropRef as &$crate::property::ToPropRef<$TYPE>;
    };
}

pub fn property_count() -> usize { PROPERTIES.read().unwrap().id2producer.len() }

/**
 * Custom properties associated with a universe.
 * Access to a registered or previously used property is `O(1)`.
 * */
#[derive(Clone, Copy)]
pub struct Prop<V: Default> {
    pub name: &'static str,
    pub index: PropertyIndex<V>,
    // ideal: "universe[debug_enabled]"
    // Unfortunately it's more like "*universe[debug_enabled].read()", although
    // "universe.get(debug_enabled)" can be used also.
}
impl<V: Default> Prop<V> {
    fn get_index(&self) -> usize { self.index.i }

    pub fn init(&mut self, producer: fn() -> PBox) {
        if self.index.i != UNSET {
            // This probably shouldn't happen.
            return;
        }
        let mut pmap = PROPERTIES.write().unwrap();
        let new_id = pmap.id2producer.len();
        let mut first_instance = false;
        self.index.i = *pmap.name2id.entry(self.name.to_string()).or_insert_with(|| {
            first_instance = true;
            new_id
        });
        if first_instance {
            pmap.id2producer.push(producer);
        }
    }
}


impl Universe {
    /// Returns a copy of the value of the given property.
    pub fn get<V: Any + Sync + Default + Clone>(&self, prop: &'static ToPropRef<V>) -> V {
        use std::sync::RwLockReadGuard;
        let v: RwLockReadGuard<V> = self[prop].read().unwrap();
        (*v).clone()
    }

    /// Sets the value of a property.
    pub fn set<V: Any + Sync + Default>(&self, prop: &'static ToPropRef<V>, val: V) {
        *self[prop].write().unwrap() = val;
    }

    /// Adds any properties that are unknown
    pub fn add_properties(&mut self) {
        let pmap = PROPERTIES.read().unwrap();
        let to_add = pmap.id2producer.len() - self.properties.len();
        if to_add <= 0 { return; } // nothing new exists in the universe
        self.properties.reserve(to_add);
        for i in self.properties.len()..pmap.id2producer.len() {
            assert!(self.properties.len() == i);
            let func_ptr = pmap.id2producer[i];
            self.properties.push(func_ptr());
            // So self.properties is a Vec of Box<Any> of RwLock<value>
        }
    }
}
impl<V: Default + Any + Sync> ::std::ops::Index<&'static ToPropRef<V>> for Universe {
    type Output = RwLock<V>;
    fn index(&self, prop: &'static ToPropRef<V>) -> &RwLock<V> {
        let prop = prop.get();
        let l: Option<&RwLock<V>> = match self.properties.get(prop.get_index()) {
            None => panic!("property #{} '{}' was never registered; perhaps there is a missed call to Universe.add_properties()?", prop.get_index(), prop.name),
            Some(v) => {
                ::desync_box(v).downcast_ref()
            },
        };
        match l {
            None => panic!("property #{} '{}' is not the expected type, {:?}", prop.get_index(), prop.name,
                "HA HA JK"),
                //self.properties.get(prop.get_index())),
            Some(ret) => ret
        }
    }
}


#[cfg(test)]
mod test {
    mod table_use {}

    use super::super::*;
    property! {
        static COMPILING_PROP ("foo"): usize; #[allow(dead_code)]
    }

    /*

    #[test]
    #[should_panic(expect = "property 'foo' was never registered")]
    fn unregistered_dynamic_property() {
        use ::Universe;
        let some_prop = Prop::<usize>::new("foo");
        let universe = Universe::new();
        universe.get(some_prop);
    }

    #[test]
    fn late_dynamic_registered_property() {
        use ::Universe;
        let universe = Universe::new();
        let some_prop = Prop::<usize>::new("foo");
        universe.get(some_prop);
    }*/

    property! { static STANDARD_PROPERTY: i64 }
    #[test]
    fn standard_property_usage() {
        let universe = Universe::new();
        assert_eq!(0, universe.get(STANDARD_PROPERTY));
        universe.set(STANDARD_PROPERTY, 20);
        assert_eq!(20, universe.get(STANDARD_PROPERTY));
    }

    property! { static PROP: usize }
    property! { static OTHER_PROP ("PROP"): usize }
    #[test]
    fn property_basics() {
        let universe = Universe::new();

        // default value
        assert!(universe.get(PROP) == 0);
        
        // set should work
        universe.set(PROP, 99);
        assert!(universe.get(PROP) == 99);

        // get from another property
        assert!(universe.get(OTHER_PROP) == 99);

        // set via another property
        universe.set(OTHER_PROP, 66);
        assert!(universe.get(PROP) == 66);
        assert!(*universe[PROP].read().unwrap() == 66);
    }

    property! { static MAP: ::std::collections::HashMap<String, i32> }

    #[test]
    fn hashmaps_with_threads() {
        let universe = Universe::new().guard();
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
                let map = universe[MAP].read().unwrap();
                println!("thread says: {}", map.get("foo").unwrap());
            })
        };
        thread.join().expect("thread failed");
        {
            let universe = universe.read().unwrap();
            let map = universe[MAP].read().unwrap();
            println!("main says: {}", map.get("foo").unwrap());
        }
    }
}
