//! Java state management and analysis, independent of LSP.

use beans_core_engine::Revision;
use beans_lang_java_model::File;

#[derive(Default)]
pub struct JavaEngine {}

impl JavaEngine {
    pub fn process(content: &str) -> File {
        beans_lang_java_semantics::lower_into(content)
    }

    pub fn store(&mut self, _revision: Revision, _model: File, _id: ()) {
        todo!("source identity and revisioned storage are not implemented")
    }

    pub fn analyse(&self, _revision: Revision, _id: ()) {
        todo!("analysis by source identity and revision is not implemented")
    }
}
