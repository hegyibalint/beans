use std::collections::HashMap;
use std::hash::Hash;

use crate::Revision;

pub struct RevisionedStorage<K: Eq + Hash, V> {
    entries: HashMap<K, Vec<Versioned<V>>>,
}

struct Versioned<V> {
    revision: Revision,
    /// The value is `None` if the entry was deleted at this revision.
    value: Option<V>,
}

impl<K: Eq + Hash, V> RevisionedStorage<K, V> {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Writes the value, handing back a reference to the now store-owned
    /// instance so the caller can keep reading it without a clone.
    pub fn put(&mut self, revision: Revision, key: K, value: V) -> &V {
        self.push_version(revision, key, Some(value))
            .as_ref()
            .expect("a put never writes a tombstone")
    }

    /// Deletion is a tombstone, so readers at older revisions still see the value.
    pub fn remove(&mut self, revision: Revision, key: K) {
        if self.entries.contains_key(&key) {
            self.push_version(revision, key, None);
        }
    }

    pub fn latest(&self, key: &K) -> Option<&V> {
        self.entries.get(key)?.last()?.value.as_ref()
    }

    /// The value as it stood at `at`: the newest version not newer than `at`.
    pub fn get(&self, key: &K, at: Revision) -> Option<&V> {
        let chain = self.entries.get(key)?;
        let newer_than_at = chain.partition_point(|v| v.revision <= at);
        chain[..newer_than_at].last()?.value.as_ref()
    }

    /// Live heads only; what index building and persistence consume.
    pub fn iter_latest(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries
            .iter()
            .filter_map(|(key, chain)| Some((key, chain.last()?.value.as_ref()?)))
    }

    /// Chains are kept in ascending revision order, so a write is an append
    /// (or an in-place replace when the same revision writes a key twice).
    fn push_version(&mut self, revision: Revision, key: K, value: Option<V>) -> &Option<V> {
        let chain = self.entries.entry(key).or_default();
        match chain.last_mut() {
            Some(head) if head.revision == revision => head.value = value,
            Some(head) => {
                debug_assert!(
                    head.revision < revision,
                    "revisions must arrive in per-key ascending order"
                );
                chain.push(Versioned { revision, value });
            }
            None => chain.push(Versioned { revision, value }),
        }
        &chain
            .last()
            .expect("the chain holds at least the version just written")
            .value
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_returns_newest_put() {
        let mut storage = RevisionedStorage::new();
        storage.put(Revision(1), "Foo.java", "v1");
        storage.put(Revision(3), "Foo.java", "v3");

        assert_eq!(storage.latest(&"Foo.java"), Some(&"v3"));
        assert_eq!(storage.latest(&"Bar.java"), None);
    }

    #[test]
    fn get_reads_the_world_as_of_a_revision() {
        let mut storage = RevisionedStorage::new();
        storage.put(Revision(1), "Foo.java", "v1");
        storage.put(Revision(3), "Foo.java", "v3");

        assert_eq!(storage.get(&"Foo.java", Revision(0)), None);
        assert_eq!(storage.get(&"Foo.java", Revision(1)), Some(&"v1"));
        assert_eq!(storage.get(&"Foo.java", Revision(2)), Some(&"v1"));
        assert_eq!(storage.get(&"Foo.java", Revision(3)), Some(&"v3"));
        assert_eq!(storage.get(&"Foo.java", Revision(9)), Some(&"v3"));
    }

    #[test]
    fn remove_hides_the_value_without_erasing_history() {
        let mut storage = RevisionedStorage::new();
        storage.put(Revision(1), "Foo.java", "v1");
        storage.remove(Revision(2), "Foo.java");

        assert_eq!(storage.latest(&"Foo.java"), None);
        assert_eq!(storage.get(&"Foo.java", Revision(2)), None);
        assert_eq!(storage.get(&"Foo.java", Revision(1)), Some(&"v1"));
    }

    #[test]
    fn remove_of_an_unknown_key_is_a_noop() {
        let mut storage: RevisionedStorage<&str, &str> = RevisionedStorage::new();
        storage.remove(Revision(1), "Foo.java");

        assert_eq!(storage.latest(&"Foo.java"), None);
    }

    #[test]
    fn put_hands_back_the_stored_instance() {
        let mut storage = RevisionedStorage::new();
        let stored = storage.put(Revision(1), "Foo.java", "v1".to_string());

        assert_eq!(stored, "v1");
    }

    #[test]
    fn same_revision_put_replaces_instead_of_stacking() {
        let mut storage = RevisionedStorage::new();
        storage.put(Revision(1), "Foo.java", "first");
        storage.put(Revision(1), "Foo.java", "second");

        assert_eq!(storage.get(&"Foo.java", Revision(1)), Some(&"second"));
    }

    #[test]
    fn iter_latest_skips_tombstones() {
        let mut storage = RevisionedStorage::new();
        storage.put(Revision(1), "Foo.java", "foo");
        storage.put(Revision(2), "Bar.java", "bar");
        storage.remove(Revision(3), "Bar.java");

        let heads: Vec<_> = storage.iter_latest().collect();
        assert_eq!(heads, [(&"Foo.java", &"foo")]);
    }
}
