use beans_core::{
    resource::{ResourceId, ResourceRoot},
    revision::Revision,
};

use crate::Engine;

impl Engine {
    pub fn document_open(
        &mut self,
        source: ResourceId,
        language_id: &str,
        contents: &str,
    ) -> Result<Option<Revision>, String> {
        self.document_process(source, language_id, contents)
    }

    pub fn document_edit(
        &mut self,
        source: ResourceId,
        language_id: &str,
        contents: &str,
    ) -> Result<Option<Revision>, String> {
        self.document_process(source, language_id, contents)
    }

    pub fn process(
        &mut self,
        source: ResourceId,
        contents: &str,
    ) -> Result<Option<Revision>, String> {
        let ResourceRoot::File { path } = &source.root else {
            return Ok(None);
        };
        if !source.entries.is_empty() || !beans_lang_java::accept(path) {
            return Ok(None);
        }
        self.document_process(source, "java", contents)
    }

    fn document_process(
        &mut self,
        source: ResourceId,
        language_id: &str,
        contents: &str,
    ) -> Result<Option<Revision>, String> {
        if language_id != "java" {
            return Ok(None);
        }
        let Some(file) = beans_lang_java::process(&source, contents) else {
            return Ok(None);
        };
        self.storage
            .publish_java(&source, &file, contents)
            .map(Some)
    }
}
