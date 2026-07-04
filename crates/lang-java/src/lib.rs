use beans_core::VirtualFile;
use beans_platform_jvm::PlatformJvm;

pub struct LanguageJava {
}

impl LanguageJava {
    pub fn open(&self, platform_jvm: &mut PlatformJvm, file: VirtualFile) {
        todo!()
    }
}

impl Default for LanguageJava {
    fn default() -> Self {
        Self {  }
    }
}
