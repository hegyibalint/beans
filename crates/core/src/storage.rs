use crate::Revision;

pub struct RevisionedStorage<T> {
    current_revision: Revision,
    models: Vec<T>,
}

impl<T> RevisionedStorage<T> {
    pub fn new() -> Self {
        Self {
            current_revision: Revision::default(),
            models: Vec::new(),
        }
    }

    pub fn put(&mut self, revision: Revision, model: T) {
        if revision > self.current_revision {
            self.current_revision = revision;
        }
        self.models.push(model);
    }
}
