use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Source {
    SourceFile {
        path: PathBuf,
    },
    ClassFile {
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
