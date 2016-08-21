
use super::Universe;
use std::any::Any;
use std::marker::PhantomData;
use std::sync::RwLock;


const UNSET: usize = ::std::usize::MAX;

/**
 * Custom properties associated with a universe.
 * Access to a registered or previously used property is `O(1)`.
 * */
pub struct Prop<V: Any + Default> {
    name: String,
    data: PhantomData<V>,
    index: RwLock<usize>,
    last_size: RwLock<usize>,
    // universe(debug_enabled)
    // debug_enable(ctx)
    // universe[debug_enabled]
}
impl<V: Any + Default> Prop<V> {
    /// Create a new property. But the property must first be registered with the `Universe`.
    pub fn new(name: &str) -> Prop<V> {
        Prop {
            name: name.to_string(),
            data: PhantomData,
            index: RwLock::new(UNSET),
            last_size: RwLock::new(0),
        }
    }
}
impl<V: Any + Default> Clone for Prop<V> {
    fn clone(&self) -> Prop<V> {
        let (index, last_size) = {
            let i = self.index.read().unwrap();
            let l = self.last_size.read().unwrap();
            (*i, *l)
        };
        Prop {
            name: self.name.clone(),
            data: PhantomData,
            index: RwLock::new(index),
            last_size: RwLock::new(last_size),
        }
    }
}
impl<'u, V: Any + Default + Copy> ::std::ops::Index<&'u Prop<V>> for Universe {
    type Output = RwLock<V>;
    fn index(&self, prop: &'u Prop<V>) -> &RwLock<V> {
        let i = *prop.index.read().unwrap();
        if i != UNSET {
            return self.properties[i].1.downcast_ref().unwrap();
        }
        // We haven't initialized.
        // But we might have a twin who has been registered.
        let mut li = prop.last_size.write().unwrap();
        while *li < self.properties.len() {
            if self.properties[*li].0 == prop.name {
                *prop.index.write().unwrap() = *li;
                return self.properties[*li].1.downcast_ref().unwrap();
            }
            *li += 1;
        }
        panic!("property '{}' was never registered", prop.name);
    }
}
impl Universe {
    /// Returns a copy of the value of the given property.
    pub fn get<V: Any + Default + Copy>(&self, prop: &Prop<V>) -> V {
        use std::ops::Deref;
        let guard = self[prop].read().unwrap();
        let v: &V = guard.deref();
        *v
    }

    /// Sets the value for a property.
    pub fn set<V: Any + Default>(&mut self, prop: &Prop<V>, val: V) {
        let mut i = *prop.index.read().unwrap();
        if i == UNSET {
            self.register(prop);
            i = *prop.index.read().unwrap();
            debug_assert!(i != UNSET);
        }
        self.properties[i].1 = Box::new(RwLock::new(val)) as Box<Any>;
    }

    /// Registers a property. Calling this multiple times with different instances of the same
    /// property is allowed, but not necessary so long as at least one has been registered.
    pub fn register<V: Any + Default>(&mut self, prop: &Prop<V>) {
        for (id, &(ref iname, _)) in self.properties.iter().enumerate() {
            if &prop.name == iname {
                // Duplicate registration is to be expected.
                *prop.index.write().unwrap() = id;
                return;
            }
        }
        *prop.index.write().unwrap() = self.properties.len();
        let v = Box::new(RwLock::new(V::default())) as Box<Any>;
        self.properties.push((prop.name.to_string(), v));
    }
}


#[cfg(test)]
mod test {
    #[test]
    #[should_panic(expect = "property 'foo' was never registered")]
    fn unregistered_property() {
        use super::*;
        use ::Universe;
        let prop = Prop::<usize>::new("foo");
        let universe = Universe::new();
        universe.get(&prop);
    }

    #[test]
    fn property() {
        use super::*;
        use ::Universe;
        let prop = Prop::<usize>::new("foo");
        let mut universe = Universe::new();
        universe.register(&prop);
        assert!(universe.get(&prop) == 0);
        universe.set(&prop, 99);
        assert!(universe.get(&prop) == 99);

        let other = Prop::<usize>::new("foo");
        assert!(universe.get(&other) == 99);

        let brother = Prop::<usize>::new("foo");
        universe.set(&brother, 66);
        assert!(universe.get(&prop) == 66);
    }
}
