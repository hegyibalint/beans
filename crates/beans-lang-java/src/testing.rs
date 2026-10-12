use beans_core::{resource::ResourceId, revision::Revision};
use beans_platform_jvm::{Class, model_query::JvmModelQuery};
use beans_storage::RevisionedStorage;

use crate::{model::File, model_query::JavaModelQuery};

#[derive(Default)]
pub(crate) struct Models {
    files: RevisionedStorage<ResourceId, File>,
    classes: RevisionedStorage<ResourceId, Class>,
}

impl Models {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn store(&mut self, revision: Revision, model: File, source: ResourceId) {
        self.files.put(revision, source, model);
    }

    pub(crate) fn file(&self, revision: Revision, source: &ResourceId) -> Option<&File> {
        self.files.get(revision, source)
    }

    pub(crate) fn remove(&mut self, revision: Revision, source: ResourceId) {
        self.files.remove(revision, source);
    }

    pub(crate) fn store_class(&mut self, revision: Revision, class: Class, source: ResourceId) {
        self.classes.put(revision, source, class);
    }

    pub(crate) fn model_query(&self, revision: Revision) -> ModelQuery<'_> {
        ModelQuery {
            files: &self.files,
            classes: Some(&self.classes),
            revision,
        }
    }
}

pub(crate) struct ModelQuery<'a> {
    files: &'a RevisionedStorage<ResourceId, File>,
    classes: Option<&'a RevisionedStorage<ResourceId, Class>>,
    revision: Revision,
}

impl<'a> ModelQuery<'a> {
    pub(crate) fn new(files: &'a RevisionedStorage<ResourceId, File>, revision: Revision) -> Self {
        Self {
            files,
            classes: None,
            revision,
        }
    }
}

impl JvmModelQuery for ModelQuery<'_> {
    fn jvm_class(&self, source: &ResourceId) -> Option<&Class> {
        self.classes?.get(self.revision, source)
    }

    fn jvm_classes(&self) -> impl Iterator<Item = (&ResourceId, &Class)> {
        self.classes
            .into_iter()
            .flat_map(|classes| classes.iter(self.revision))
    }
}

impl JavaModelQuery for ModelQuery<'_> {
    fn java_file(&self, source: &ResourceId) -> Option<&File> {
        self.files.get(self.revision, source)
    }

    fn java_files(&self) -> impl Iterator<Item = (&ResourceId, &File)> {
        self.files.iter(self.revision)
    }
}
