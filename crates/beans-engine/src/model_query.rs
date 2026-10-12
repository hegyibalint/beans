use beans_core::resource::ResourceId;
use beans_lang_java::{model::File, model_query::JavaModelQuery};
use beans_platform_jvm::{Class, model_query::JvmModelQuery};

use crate::storage::{Model, Storage};

pub(crate) struct ModelQuery<'a> {
    storage: &'a Storage,
}

impl<'a> ModelQuery<'a> {
    pub(crate) fn new(storage: &'a Storage) -> Self {
        Self { storage }
    }
}

impl JavaModelQuery for ModelQuery<'_> {
    fn java_file(&self, source: &ResourceId) -> Option<&File> {
        match &self.storage.get(source)?.model {
            Model::Java(file) => Some(file),
            Model::Jvm(_) => None,
        }
    }

    fn java_files(&self) -> impl Iterator<Item = (&ResourceId, &File)> {
        self.storage
            .iter()
            .filter_map(|(source, entry)| match &entry.model {
                Model::Java(file) => Some((source, file)),
                Model::Jvm(_) => None,
            })
    }
}

impl JvmModelQuery for ModelQuery<'_> {
    fn jvm_class(&self, id: &ResourceId) -> Option<&Class> {
        match &self.storage.get(id)?.model {
            Model::Java(_) => None,
            Model::Jvm(class) => Some(class),
        }
    }
}

#[cfg(test)]
mod tests;
