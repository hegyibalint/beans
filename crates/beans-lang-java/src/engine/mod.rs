//! Java processing, independent of persistence and editor protocols.

use std::path::Path;

use crate::{lowering::lower_into, model::File};

#[derive(Default)]
pub struct JavaLanguage;

impl JavaLanguage {
    pub fn new() -> Self {
        Self
    }

    pub fn accept(&self, path: &Path) -> bool {
        path.extension()
            .is_some_and(|extension| extension == "java")
    }

    pub fn process(&self, contents: &str) -> File {
        lower_into(contents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accept_recognizes_java_source_paths() {
        let java = JavaLanguage::new();
        assert!(java.accept(Path::new("src/Example.java")));
        assert!(!java.accept(Path::new("README.md")));
        assert!(!java.accept(Path::new("Example.class")));
    }

    #[test]
    fn process_lowers_java_source_without_storage() {
        let file = JavaLanguage::new().process("package example; class Example {}");
        assert_eq!(file.package_name.as_slice(), ["example"]);
    }
}
