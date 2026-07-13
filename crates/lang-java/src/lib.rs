mod diagnostics;
mod model;
mod parser;
mod projection;

use std::collections::HashMap;

use beans_core::Revision;
use beans_core::analysis::FileAnalysis;
use beans_core::storage::RevisionedStorage;
use beans_platform_jvm::PlatformJvm;

use crate::diagnostics::dummy_diagnostics;
use crate::model::JavaFile;
use crate::parser::JavaParser;
use crate::projection::project;

pub struct LanguageJava {
    parser: JavaParser,
    model_store: RevisionedStorage<HashMap<String, JavaFile>>,
}

impl LanguageJava {
    pub fn new() -> LanguageJava {
        LanguageJava {
            parser: JavaParser::new(),
            model_store: RevisionedStorage::new(),
        }
    }

    pub fn process(
        &mut self,
        revision: Revision,
        _platform_jvm: &mut PlatformJvm,
        _uri: &str,
        contents: &str,
    ) {
        let java_model = self.parser.parse(contents);
        self.model_store.put(revision, java_model.clone());
        project(java_model)
            .iter()
            .for_each(|jvm_class| _platform_jvm.register(jvm_class));
    }

    pub fn analyze(&mut self, revision: Revision, uri: &str) -> FileAnalysis {
        let model = self.model_store.get(revision);
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
