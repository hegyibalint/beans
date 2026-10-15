use beans_core::resource::ResourceId;

use crate::model::File;

/// The provider selects model versions; Java only interprets the returned models.
pub trait JavaModelQuery {
    fn java_file(&self, source: &ResourceId) -> Result<Option<File>, String>;

    fn java_files(&self) -> Result<Vec<(ResourceId, File)>, String>;
}
