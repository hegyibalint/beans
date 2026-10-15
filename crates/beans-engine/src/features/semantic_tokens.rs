use beans_core::{ranges::ByteRange, resource::ResourceId};

use crate::Engine;

impl Engine {
    pub fn type_tokens(&self, source: &ResourceId) -> Vec<ByteRange> {
        self.java.type_tokens(self.revision, source)
    }
}
