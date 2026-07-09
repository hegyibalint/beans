use beans_core::{Revision, analysis::FileAnalysis};
use beans_lang_java::LanguageJava;
use beans_platform_jvm::PlatformJvm;

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

#[allow(unused_variables)]
impl Beans {
    pub fn process(&mut self, uri: &str, contents: &str) -> FileAnalysis {
        let current_revision = self.revision.bump();

        if uri.ends_with(".java") {
            return self
                .lang_java
                .process(self.revision, &mut self.platform_jvm, uri, contents);
        } else {
            panic!("unsupported file type: {}", uri);
        }
    }
}
