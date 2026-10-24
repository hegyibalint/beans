use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct TempDir {
    root: tempfile::TempDir,
}

impl TempDir {
    pub fn new() -> Self {
        Self {
            root: tempfile::tempdir().unwrap(),
        }
    }

    pub fn root(&self) -> &Path {
        self.root.path()
    }

    pub fn path(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.root().join(relative)
    }

    pub fn write(&self, relative: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> PathBuf {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, contents).unwrap();
        path
    }
}

impl Default for TempDir {
    fn default() -> Self {
        Self::new()
    }
}
