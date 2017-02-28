//! A quick-and-dirty bimap implementation.

use std::collections::HashMap;
use std::collections::hash_map::Keys;
use std::hash::Hash;
use std::borrow::Borrow;

pub struct BiMap<K: Eq + Clone + Hash, V: Eq + Clone + Hash> {
    forward: HashMap<K, V>,
    reverse: HashMap<V, K>,
}
impl<K: Eq + Clone + Hash, V: Eq + Clone + Hash> BiMap<K, V> {
    pub fn new() -> Self {
        BiMap {
            forward: HashMap::new(),
            reverse: HashMap::new(),
        }
    }
    pub fn insert(&mut self, key: K, val: V) {
        self.forward.insert(key.clone(), val.clone());
        self.reverse.insert(val, key);
    }

    pub fn get_forward<Q: ?Sized>(&self, key: &Q) -> Option<&V>
        where K: Borrow<Q>, Q: Hash + Eq
    {
        self.forward.get(key)
    }

    pub fn get_reverse<R: ?Sized>(&self, val: &R) -> Option<&K>
        where V: Borrow<R>, R: Hash + Eq
    {
        self.reverse.get(val)
    }

    pub fn clear(&mut self) {
        self.forward.clear();
        self.reverse.clear();
    }

    pub fn keys(&self) -> Keys<K, V> {
        self.forward.keys()
    }

    pub fn iter(&self) -> ::std::collections::hash_map::Iter<K, V> {
        self.forward.iter()
    }
}
impl<K: Eq + Clone + Hash, V: Eq + Clone + Hash> Default for BiMap<K, V> {
    fn default() -> Self {
        BiMap::new()
    }
}

