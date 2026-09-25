use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Source {
    Source {
        uri: String,
    },
    Class {
        path: PathBuf,
    },
    JarEntry {
        jar_path: PathBuf,
        entry_path: String,
    },
    JmodEntry {
        jmod_path: PathBuf,
        entry_path: String,
    },
    JimageEntry {
        jimage_path: PathBuf,
        entry_path: String,
    },
}

/// A byte position in a source known to the engine.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    pub source: Source,
    pub offset: usize,
}

impl SourceLocation {
    pub fn new(source: Source, offset: usize) -> Self {
        Self { source, offset }
    }
}

impl Source {
    pub fn uri(uri: impl Into<String>) -> Self {
        Self::Source { uri: uri.into() }
    }

    pub fn class_file(path: impl Into<PathBuf>) -> Self {
        Self::Class { path: path.into() }
    }

    pub fn jar_entry(jar_path: impl Into<PathBuf>, entry_path: impl Into<String>) -> Self {
        Self::JarEntry {
            jar_path: jar_path.into(),
            entry_path: entry_path.into(),
        }
    }

    pub fn jmod_entry(jmod_path: impl Into<PathBuf>, entry_path: impl Into<String>) -> Self {
        Self::JmodEntry {
            jmod_path: jmod_path.into(),
            entry_path: entry_path.into(),
        }
    }

    pub fn jimage_entry(jimage_path: impl Into<PathBuf>, entry_path: impl Into<String>) -> Self {
        Self::JimageEntry {
            jimage_path: jimage_path.into(),
            entry_path: entry_path.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Source, SourceLocation};

    #[test]
    fn source_files_and_virtual_documents_share_a_uri_based_kind() {
        assert_eq!(
            Source::uri("file:///Example.java"),
            Source::Source {
                uri: "file:///Example.java".into(),
            }
        );
        assert_eq!(
            Source::uri("untitled:Example.java"),
            Source::Source {
                uri: "untitled:Example.java".into(),
            }
        );
        assert_ne!(
            Source::uri("file:///Example.class"),
            Source::class_file("Example.class")
        );
        assert_eq!(
            Source::jar_entry("library.jar", "p/Example.class"),
            Source::JarEntry {
                jar_path: "library.jar".into(),
                entry_path: "p/Example.class".into(),
            }
        );
    }

    #[test]
    fn locations_pair_a_source_with_a_byte_offset() {
        let source = Source::uri("file:///Example.java");

        assert_eq!(
            SourceLocation::new(source.clone(), 17),
            SourceLocation { source, offset: 17 }
        );
    }
}
