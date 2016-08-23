
use std::marker::PhantomData;
use std::any::Any;
use std::sync::RwLock;
use std::collections::HashMap;

use super::{Universe, PropertyValue};

struct GlobalProperties {
    name2id: HashMap<String, usize>,
    id2default: Vec<Box<PropertyValue>>,
}

lazy_static! {
    static ref PROPERTIES: RwLock<GlobalProperties> = RwLock::new(GlobalProperties {
        name2id: HashMap::new(),
        id2default: Vec::new(),
    });
}

#[derive(Copy, Clone, PartialEq)]
pub struct PropertyIndex<V> {
    pub i: usize,
    pub v: PhantomData<V>,
}


use std::usize;
pub const UNSET: usize = usize::MAX;

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
            use $crate::property::*;
            constructor! { init }
            #[allow(dead_code)]
            extern fn init() {
                unsafe { VAL.init(); }
            }
            // Can't access this directly because 'static mut' is unsafe to touch in any way.
            static mut VAL: Prop<'static, $TYPE> = Prop {
                name: $NAME_STR,
                index: PropertyIndex {
                    i: UNSET,
                    v: ::std::marker::PhantomData,
                }
            };

            #[derive(Clone, Copy)]
            pub struct PropRef;
            impl ::std::ops::Deref for PropRef {
                type Target = Prop<'static, $TYPE>;
                fn deref(&self) -> &Self::Target {
                    unsafe { &VAL }
                }
            }

        }

        $(#[$ATTR])*
        pub static $NAME: $NAME::PropRef = $NAME::PropRef;
    };
}

// property! { stinky: f64 }
// property! { cheeses: Vec<i32> named "cheese_list" }

pub fn property_count() -> usize { PROPERTIES.read().unwrap().id2default.len() }

/**
 * Custom properties associated with a universe.
 * Access to a registered or previously used property is `O(1)`.
 * */
#[derive(Clone, Copy)]
pub struct Prop<'a, V: PropertyValue> {
    pub name: &'a str,
    pub index: PropertyIndex<V>,
    // ideal: "universe[debug_enabled]"
    // Unfortunately it's more like "*universe[debug_enabled].read()", although
    // "universe.get(debug_enabled)" can be used also.
}
impl<'a, V: PropertyValue + Default> Prop<'a, V> {
    fn get_index(&self) -> usize { self.index.i }

    pub fn init(&mut self) {
        if self.index.i != UNSET {
            // This probably shouldn't happen.
            return;
        }
        let mut pmap = PROPERTIES.write().unwrap();
        let new_id = pmap.id2default.len();
        let mut first_instance = false;
        self.index.i = *pmap.name2id.entry(self.name.to_string()).or_insert_with(|| {
            first_instance = true;
            new_id
        });
        if first_instance {
            pmap.id2default.push(Box::new(V::default()) as Box<PropertyValue>);
        }
    }
}


impl<'a, V: PropertyValue + Default> ::std::ops::Index<Prop<'a, V>> for Universe {
    type Output = RwLock<V>;
    fn index(&self, prop: Prop<'a, V>) -> &RwLock<V> {
        let l: Option<&RwLock<V>> = match self.properties.get(prop.get_index()) {
            None => panic!("property #{} '{}' was never registered; perhaps there is a missed call to Universe.add_properties()?", prop.get_index(), prop.name),
            Some(v) => v.downcast_ref(),
        };
        match l {
            None => panic!("property #{} '{}' is not the expected type, {:?}", prop.get_index(), prop.name,
                self.properties.get(prop.get_index())),
            Some(ret) => ret
        }
    }
}
impl Universe {
    /// Returns a copy of the value of the given property.
    pub fn get<V: PropertyValue + Default + Clone>(&self, prop: Prop<V>) -> V {
        use std::sync::RwLockReadGuard;
        let v: RwLockReadGuard<V> = self[prop].read().unwrap();
        (*v).clone()
    }

    /// Sets the value of a property.
    pub fn set<V: PropertyValue + Default>(&self, prop: Prop<V>, val: V) {
        *self[prop].write().unwrap() = val;
    }

    /// Adds any properties that are unknown
    pub fn add_properties(&mut self) {
        let pmap = PROPERTIES.read().unwrap();
        let to_add = pmap.id2default.len() - self.properties.len();
        if to_add <= 0 { return; } // nothing new exists in the universe
        self.properties.reserve(to_add);
        for i in self.properties.len()..pmap.id2default.len() {
            assert!(self.properties.len() == i);
            let l: Box<Any> = pmap.id2default[i].dupe_locked();
            self.properties.push(l);
            // So self.properties is a Vec of Box<Any> of RwLock<PropertyValue>
        }
    }
}


#[cfg(test)]
mod test {
    property! {
        static COMPILING_PROP ("foo"): usize; #[allow(dead_code)]
    }

    /*
    use super::*;


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
        use ::Universe;
        let universe = Universe::new();
        assert_eq!(0, universe.get(*STANDARD_PROPERTY));
        universe.set(*STANDARD_PROPERTY, 20);
        assert_eq!(20, universe.get(*STANDARD_PROPERTY));
    }

    property! { static PROP: usize }
    property! { static OTHER_PROP ("PROP"): usize }
    #[test]
    fn property_basics() {
        use ::Universe;
        let universe = Universe::new();

        // default value
        assert!(universe.get(*PROP) == 0);
        
        // set should work
        universe.set(*PROP, 99);
        assert!(universe.get(*PROP) == 99);

        // get from another property
        assert!(universe.get(*OTHER_PROP) == 99);

        // set via another property
        universe.set(*OTHER_PROP, 66);
        assert!(universe.get(*PROP) == 66);
    }
}
