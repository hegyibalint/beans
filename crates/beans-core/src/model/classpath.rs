use std::path::{Component, Path, PathBuf};

#[derive(Default)]
pub struct Classpath {
    elements: Vec<ClasspathElement>,
}

impl Classpath {
    pub fn new(elements: Vec<ClasspathElement>) -> Self {
        Self { elements }
    }

    /// Tests source-root membership, not artifact selection. Paths use the same base.
    pub fn contains_source_file(&self, path: &Path) -> bool {
        if path
            .components()
            .any(|component| component == Component::ParentDir)
        {
            return false;
        }
        self.elements.iter().any(|element| {
            let root = element.path.strip_prefix(".").unwrap_or(&element.path);
            let path = path.strip_prefix(".").unwrap_or(path);
            if path.is_absolute() == root.is_absolute() {
                return path.starts_with(root);
            }
            std::env::current_dir().ok().is_some_and(|cwd| {
                let absolute_path = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    cwd.join(path)
                };
                let absolute_root = if root.is_absolute() {
                    root.to_path_buf()
                } else {
                    cwd.join(root)
                };
                absolute_path.starts_with(absolute_root)
            })
        })
    }
}

pub struct ClasspathElement {
    path: PathBuf,
    hash: blake3::Hash,
}

impl ClasspathElement {
    pub fn new(path: PathBuf, hash: blake3::Hash) -> Self {
        Self { path, hash }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_roots_match_path_components_not_string_prefixes() {
        let classpath = Classpath::new(vec![ClasspathElement::new("src".into(), [0; 32].into())]);

        assert!(classpath.contains_source_file(Path::new("src/p/Example.java")));
        assert!(classpath.contains_source_file(Path::new("./src/Example.java")));
        assert!(
            classpath
                .contains_source_file(&std::env::current_dir().unwrap().join("src/p/Example.java"))
        );
        assert!(!classpath.contains_source_file(Path::new("src-other/Example.java")));
        assert!(!classpath.contains_source_file(Path::new("other/Example.java")));
        assert!(!classpath.contains_source_file(Path::new("src/../other/Example.java")));
    }

    #[test]
    fn empty_classpath_excludes_all_sources() {
        assert!(!Classpath::default().contains_source_file(Path::new("Example.java")));
    }

    #[test]
    fn dot_root_includes_sources_relative_to_the_base() {
        let classpath = Classpath::new(vec![ClasspathElement::new(".".into(), [0; 32].into())]);

        assert!(classpath.contains_source_file(Path::new("Example.java")));
        assert!(classpath.contains_source_file(Path::new("src/Example.java")));
    }
}
