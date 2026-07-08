use beans_core::{analysis::FileAnalysis, Revision, VirtualFile};
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
            lang_java: LanguageJava::new()
        }
    }

    /// The revision the world is currently at.
    pub fn revision(&self) -> Revision {
        self.revision
    }
}

#[allow(unused_variables)]
impl Beans {
    pub fn open(&mut self, uri: &str, contents: &str) -> FileAnalysis {
        if uri.ends_with(".java") {
            return self.lang_java.open(self.revision, &mut self.platform_jvm, uri, contents);
        } else {
            panic!("unsupported file type: {}", uri);
        }
    }
}
