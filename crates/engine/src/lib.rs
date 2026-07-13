use beans_core::{Revision, analysis::FileAnalysis};
use beans_lang_java::LanguageJava;
use beans_platform_jvm::{PlatformJvm, model::JvmSource};

pub struct Beans {
    revision: Revision,
    platform_jvm: PlatformJvm,
    lang_java: LanguageJava,
}

impl Beans {
    pub fn new() -> Beans {
        Beans {
            revision: Revision::default(),
            platform_jvm: PlatformJvm::new(),
            lang_java: LanguageJava::new(),
        }
    }
}

impl Beans {
    /// `None` when no language claims the source; the editor sends us
    /// all kinds of files, and skipping them is not an error.
    pub fn process(&mut self, source: JvmSource, contents: &str) -> Option<FileAnalysis> {
        let revision = self.revision.bump();

        if self.lang_java.accepts(&source) {
            self.lang_java
                .process(source.clone(), revision, &mut self.platform_jvm, contents);
            return self.lang_java.analyze(&source, revision);
        }

        None
    }
}
