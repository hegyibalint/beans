use std::collections::HashMap;

use crate::Revision;

pub struct RevisionedStorage<T> {
    revisions: HashMap<Revision, Vec<T>>
}

impl<T> RevisionedStorage<T> {
    pub fn new() -> Self {
        Self {
            revisions: HashMap::new(),
        }
    }

    pub fn put(&mut self, revision: Revision, model: T) {
        self.revisions.entry(revision).or_default().push(model)
    }
}
