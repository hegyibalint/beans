use beans_core::resource::ResourceId;

use crate::model::File;

/// The provider selects model versions; Java only interprets the returned models.
pub trait JavaModelQuery {
    fn java_file(&self, source: &ResourceId) -> Option<&File>;

    fn java_files(&self) -> impl Iterator<Item = (&ResourceId, &File)>;
}
