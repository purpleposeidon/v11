
use super::{Storable, Universe};
use std::any::Any;
use std::marker::PhantomData;
use std::sync::RwLock;


const UNSET: usize = ::std::usize::MAX;

// TODO: docs
pub struct Prop<V: Storable + Any> {
    name: String,
    data: PhantomData<V>,
    index: RwLock<usize>,
    last_size: RwLock<usize>,
    // universe(debug_enabled)
    // debug_enable(ctx)
    // universe[debug_enabled]
}
impl<V: Storable + Any> Prop<V> {
    pub fn new(name: &str) -> Prop<V> {
        Prop {
            name: name.to_string(),
            data: PhantomData,
            index: RwLock::new(UNSET),
            last_size: RwLock::new(0),
        }
    }
}
impl Universe {
    pub fn get<V: Storable + Any>(&self, prop: &Prop<V>) -> V {
        let i = *prop.index.read().unwrap();
        if i != UNSET {
            let b: &V = self.properties[i].1.downcast_ref().unwrap();
            return *b;
        }
        // We haven't initialized.
        // But we might have a twin who has been.
        let mut li = prop.last_size.write().unwrap();
        while *li < self.properties.len() {
            if self.properties[*li].0 == prop.name {
                *prop.index.write().unwrap() = *li;
                let b: &V = self.properties[*li].1.downcast_ref().unwrap();
                return *b;
            }
            *li += 1;
        }
        V::default()
    }

    pub fn set<V: Storable + Any>(&mut self, prop: &Prop<V>, val: V) {
        let mut i = *prop.index.read().unwrap();
        if i == UNSET {
            self.register(prop);
            i = *prop.index.read().unwrap();
            debug_assert!(i != UNSET);
        }
        self.properties[i].1 = Box::new(val) as Box<Any>;
    }

    pub fn register<V: Storable + Any>(&mut self, prop: &Prop<V>) {
        for (id, &(ref iname, _)) in self.properties.iter().enumerate() {
            if &prop.name == iname {
                *prop.index.write().unwrap() = id;
                return;
            }
        }
        *prop.index.write().unwrap() = self.properties.len();
        let v = Box::new(V::default()) as Box<Any>;
        self.properties.push((prop.name.to_string(), v));
    }
}


#[cfg(test)]
mod test {
#[test]
    fn property() {
        use super::*;
        use ::Universe;
        let prop = Prop::<usize>::new("foo");
        let mut universe = Universe::new();
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
