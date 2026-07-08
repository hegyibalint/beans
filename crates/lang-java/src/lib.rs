pub mod model;
mod parser;
mod diagnostics;

use beans_core::analysis::FileAnalysis;
use beans_core::{Revision, VirtualFile};
use beans_core::storage::RevisionedStorage;
use beans_platform_jvm::PlatformJvm;

use crate::diagnostics::dummy_diagnostics;
use crate::model::JavaFile;
use crate::parser::JavaParser;

pub struct LanguageJava {
    parser: JavaParser,
    model_store: RevisionedStorage<JavaFile>
}

impl LanguageJava {
    pub fn new() -> LanguageJava {
        LanguageJava {
            parser: JavaParser::new(),
            model_store: RevisionedStorage::new()
        }
    }

    pub fn open(&mut self, revision: Revision, _platform_jvm: &mut PlatformJvm, uri: &str, contents: &str) -> FileAnalysis {
        let model = self.parser.parse(contents);
        self.model_store.put(revision, model.clone());
        FileAnalysis {
            diagnostics: vec![dummy_diagnostics(&model)],
            actions: vec![],
        }
    }
}

impl Default for LanguageJava {
    fn default() -> Self {
        Self::new()
    }
}
