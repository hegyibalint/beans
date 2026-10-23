mod jdk_override;
mod loading;
mod paths;
mod schema;

use std::path::{Path, PathBuf};

use beans_workspace_toml::{model::Project, parse};

struct Root(tempfile::TempDir);

impl Root {
    fn new() -> Self {
        Self(tempfile::tempdir().unwrap())
    }

    fn path(&self) -> &Path {
        self.0.path()
    }

    fn descriptor(&self) -> PathBuf {
        self.path().join(beans_workspace_toml::DESCRIPTOR)
    }
}

fn project(contents: &str) -> Project {
    parse(contents, Path::new("/project"))
        .unwrap()
        .model()
        .clone()
}
