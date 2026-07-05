use beans_core::{FileId, Revision, VirtualFile};
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
    pub fn open(&mut self, file: VirtualFile) {
        if file.uri.ends_with(".java") {
            self.lang_java.open(&mut self.platform_jvm, file);
        } else {
            panic!("unsupported file type: {}", file.uri);
        }
    }

    pub fn change(&mut self, file: FileId, text: String) {
        todo!("bump revision; re-translate + re-project; diff old vs new model into the indices")
    }

    pub fn close(&mut self, file: FileId) {
        todo!("drop the editor overlay; keep the file's indexed contributions")
    }
}
