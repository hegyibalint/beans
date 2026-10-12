use beans_core::resource::ResourceId;

use crate::Class;

/// The provider selects model versions; JVM only interprets the returned models.
pub trait JvmModelQuery {
    fn jvm_class(&self, id: &ResourceId) -> Option<&Class>;
}
