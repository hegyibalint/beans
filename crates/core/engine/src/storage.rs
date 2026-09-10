use std::{collections::HashMap, hash::Hash};

use crate::Revision;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RevisionEntry<K> {
    pub revision: Revision,
    pub key: K,
}

pub struct RevisionedStorage<K, V> {
    entries: HashMap<K, Vec<Version<V>>>,
}

struct Version<V> {
    revision: Revision,
    value: Option<V>,
}

impl<K, V> Default for RevisionedStorage<K, V> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

impl<K: Eq + Hash, V> RevisionedStorage<K, V> {
    pub fn put(&mut self, revision: Revision, key: K, value: V) -> RevisionEntry<K>
    where
        K: Clone,
    {
        self.write(revision, key.clone(), Some(value));
        RevisionEntry { revision, key }
    }

    pub fn get(&self, revision: Revision, key: &K) -> Option<&V> {
        let versions = self.entries.get(key)?;
        let end = versions.partition_point(|version| version.revision <= revision);
        versions[..end].last()?.value.as_ref()
    }

    pub fn iter(&self, revision: Revision) -> impl Iterator<Item = (&K, &V)> {
        self.entries
            .keys()
            .filter_map(move |key| self.get(revision, key).map(|value| (key, value)))
    }

    pub fn remove(&mut self, revision: Revision, key: K) {
        if self.entries.contains_key(&key) {
            self.write(revision, key, None);
        }
    }

    fn write(&mut self, revision: Revision, key: K, value: Option<V>) {
        let versions = self.entries.entry(key).or_default();
        if let Some(head) = versions.last_mut() {
            assert!(
                head.revision <= revision,
                "revisions must be non-decreasing per key"
            );
            if head.revision == revision {
                head.value = value;
                return;
            }
        }
        versions.push(Version { revision, value });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_returns_an_owned_address_of_the_written_value() {
        let mut storage = RevisionedStorage::default();
        let entry = storage.put(Revision(2), String::from("A"), "first");
        storage.put(Revision(5), String::from("A"), "second");

        assert_eq!(entry.revision, Revision(2));
        assert_eq!(entry.key, "A");
        assert_eq!(storage.get(entry.revision, &entry.key), Some(&"first"));
    }

    #[test]
    fn iteration_yields_only_live_values_at_the_requested_revision() {
        let mut storage = RevisionedStorage::default();
        storage.put(Revision(2), "A", "first");
        storage.put(Revision(3), "B", "other");
        storage.put(Revision(4), "A", "second");
        storage.remove(Revision(5), "B");

        for (revision, expected) in [
            (1, vec![]),
            (2, vec![("A", "first")]),
            (3, vec![("A", "first"), ("B", "other")]),
            (4, vec![("A", "second"), ("B", "other")]),
            (5, vec![("A", "second")]),
            (9, vec![("A", "second")]),
        ] {
            let mut actual: Vec<_> = storage.iter(Revision(revision)).map(|(k, v)| (*k, *v)).collect();
            actual.sort();
            assert_eq!(actual, expected);
        }
    }

    #[test]
    fn unknown_key_has_no_value() {
        let storage = RevisionedStorage::<&str, &str>::default();

        assert_eq!(storage.get(Revision(5), &"missing"), None);
    }

    #[test]
    fn get_returns_the_newest_value_not_newer_than_the_requested_revision() {
        let mut storage = RevisionedStorage::default();
        storage.put(Revision(2), "A", "first");
        storage.put(Revision(5), "A", "second");

        for (revision, expected) in [
            (1, None),
            (2, Some(&"first")),
            (4, Some(&"first")),
            (5, Some(&"second")),
            (9, Some(&"second")),
        ] {
            assert_eq!(storage.get(Revision(revision), &"A"), expected);
        }
    }

    #[test]
    fn keys_have_independent_histories_and_can_share_a_revision() {
        let mut storage = RevisionedStorage::default();
        storage.put(Revision(2), "A", "old A");
        storage.put(Revision(5), "A", "new A");
        storage.put(Revision(3), "B", "old B");
        storage.put(Revision(5), "B", "new B");

        assert_eq!(storage.get(Revision(4), &"A"), Some(&"old A"));
        assert_eq!(storage.get(Revision(4), &"B"), Some(&"old B"));
        assert_eq!(storage.get(Revision(5), &"A"), Some(&"new A"));
        assert_eq!(storage.get(Revision(5), &"B"), Some(&"new B"));
    }

    #[test]
    fn same_revision_writes_replace_rather_than_append() {
        let mut storage = RevisionedStorage::default();
        storage.put(Revision(1), "A", "old");
        storage.put(Revision(2), "A", "first");
        storage.put(Revision(2), "A", "replacement");
        assert_eq!(storage.get(Revision(2), &"A"), Some(&"replacement"));

        storage.remove(Revision(2), "A");
        assert_eq!(storage.get(Revision(2), &"A"), None);

        storage.put(Revision(2), "A", "restored");
        assert_eq!(storage.get(Revision(2), &"A"), Some(&"restored"));
        assert_eq!(storage.get(Revision(1), &"A"), Some(&"old"));
        assert_eq!(storage.entries["A"].len(), 2);
    }

    #[test]
    fn deletion_hides_the_value_without_erasing_history() {
        let mut storage = RevisionedStorage::default();
        storage.put(Revision(1), "A", "value");
        storage.remove(Revision(3), "A");

        assert_eq!(storage.get(Revision(2), &"A"), Some(&"value"));
        assert_eq!(storage.get(Revision(3), &"A"), None);
        assert_eq!(storage.get(Revision(9), &"A"), None);
    }

    #[test]
    fn a_later_put_reintroduces_a_deleted_key() {
        let mut storage = RevisionedStorage::default();
        storage.put(Revision(1), "A", "old");
        storage.remove(Revision(3), "A");
        storage.put(Revision(5), "A", "new");

        assert_eq!(storage.get(Revision(2), &"A"), Some(&"old"));
        assert_eq!(storage.get(Revision(4), &"A"), None);
        assert_eq!(storage.get(Revision(5), &"A"), Some(&"new"));
    }

    #[test]
    fn removing_an_unknown_key_is_a_noop() {
        let mut storage = RevisionedStorage::<&str, &str>::default();
        storage.remove(Revision(2), "missing");

        assert_eq!(storage.get(Revision(2), &"missing"), None);
        assert!(storage.entries.is_empty());
    }

    #[test]
    #[should_panic(expected = "revisions must be non-decreasing per key")]
    fn put_rejects_a_revision_older_than_the_keys_latest_write() {
        let mut storage = RevisionedStorage::default();
        storage.put(Revision(5), "A", "new");
        storage.put(Revision(4), "A", "old");
    }

    #[test]
    #[should_panic(expected = "revisions must be non-decreasing per key")]
    fn remove_rejects_a_revision_older_than_the_keys_latest_write() {
        let mut storage = RevisionedStorage::default();
        storage.put(Revision(5), "A", "value");
        storage.remove(Revision(4), "A");
    }

    #[test]
    fn keys_need_no_default_and_values_need_neither_default_nor_clone() {
        #[derive(Clone, PartialEq, Eq, Hash)]
        struct Key(u64);
        struct Value(u64);

        let mut storage = RevisionedStorage::default();
        storage.put(Revision(1), Key(7), Value(42));

        assert_eq!(storage.get(Revision(1), &Key(7)).unwrap().0, 42);
    }
}
