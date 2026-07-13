use std::collections::HashMap;
use std::hash::Hash;

use crate::Revision;

pub struct RevisionedStorage<V> {
    current_revision: Revision,
    models: Vec<V>,
}

impl<V> RevisionedStorage<V> {
    pub fn new() -> Self {
        Self {
            current_revision: Revision::default(),
            models: Vec::new(),
        }
    }

    pub fn put(&mut self, revision: Revision, model: V) {
        if revision > self.current_revision {
            self.current_revision = revision;
        }
        self.models.push(model);
    }
}

pub struct Index<K, V> {
    index: HashMap<K, V>,
}

impl<K: Eq + Hash, V> Index<K, V> {
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.index.insert(key, value);
    }
}
