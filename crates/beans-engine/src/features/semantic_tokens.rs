use beans_core::{location::Location, ranges::ByteRange};

use crate::Engine;

impl Engine {
    pub fn type_tokens(&self, source: &Location) -> Vec<ByteRange> {
        self.java.type_tokens(self.revision, source)
    }
}
