use beans_core::{
    engine::Revision,
    model::source::{Source, SourceSpan},
};

use crate::engine::JavaEngine;

impl JavaEngine {
    /// Finds a declaration from a Java source position in the current revision.
    pub fn goto_declaration(
        &self,
        revision: Revision,
        source: &Source,
        offset: usize,
    ) -> Option<SourceSpan> {
        let _file = self.file(revision, source)?;
        let _ = offset;
        todo!("Java go-to-declaration")
    }
}
