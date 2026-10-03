use std::path::PathBuf;

/// Identifies an input contributing models, not a symbol declared within it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Origin {
    Document {
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

impl Origin {
    pub fn uri(uri: impl Into<String>) -> Self {
        Self::Document { uri: uri.into() }
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
    use super::Origin;

    #[test]
    fn source_files_and_virtual_documents_share_a_uri_based_kind() {
        assert_eq!(
            Origin::uri("file:///Example.java"),
            Origin::Document {
                uri: "file:///Example.java".into(),
            }
        );
        assert_eq!(
            Origin::uri("untitled:Example.java"),
            Origin::Document {
                uri: "untitled:Example.java".into(),
            }
        );
        assert_ne!(
            Origin::uri("file:///Example.class"),
            Origin::class_file("Example.class")
        );
        assert_eq!(
            Origin::jar_entry("library.jar", "p/Example.class"),
            Origin::JarEntry {
                jar_path: "library.jar".into(),
                entry_path: "p/Example.class".into(),
            }
        );
    }
}
