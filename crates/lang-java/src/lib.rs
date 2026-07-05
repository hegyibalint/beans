pub mod model;
mod parser;

use beans_core::VirtualFile;
use beans_platform_jvm::PlatformJvm;

use crate::parser::JavaParser;

pub struct LanguageJava {
    parser: JavaParser
}

impl LanguageJava {
    pub fn new() -> LanguageJava {
        LanguageJava { parser: JavaParser::new() }
    }

    pub fn open(&mut self, platform_jvm: &mut PlatformJvm, file: VirtualFile) {
        let _model = self.parser.parse(file);
    }
}
