use std::collections::HashMap;

use beans_core::resource::ResourceId;

pub struct Storage {
    entries: HashMap<ResourceId, Entry>,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn get(&self, source: &ResourceId) -> Option<&Entry> {
        self.entries.get(source)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&ResourceId, &Entry)> {
        self.entries.iter()
    }

    pub fn insert(&mut self, source: ResourceId, entry: Entry) -> Option<Entry> {
        self.entries.insert(source, entry)
    }

    pub fn remove(&mut self, source: &ResourceId) -> Option<Entry> {
        self.entries.remove(source)
    }
}

pub struct Entry {
    pub model: Model,
    pub content: Option<Content>,
}

pub enum Model {
    Java(beans_lang_java::model::File),
    Jvm(beans_platform_jvm::Class),
}

pub enum Content {
    Stored(Vec<u8>),
}

#[cfg(test)]
mod tests;
