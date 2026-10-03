use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::parser::{DESCRIPTOR, LoadError, ParseError, load};

struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "beans-workspace-toml-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }

    fn descriptor(&self) -> PathBuf {
        self.0.join(DESCRIPTOR)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn an_absent_descriptor_is_not_an_error() {
    let root = Root::new();

    assert!(load(root.path()).unwrap().is_none());
}

#[test]
fn a_descriptor_is_resolved_against_its_containing_root() {
    let root = Root::new();
    fs::write(root.descriptor(), "[unit.app]\nsources = [\"src\"]\n").unwrap();

    let workspace = load(root.path()).unwrap().unwrap();

    assert_eq!(
        workspace.model().units["app"].sources,
        [root.path().join("src")]
    );
}

#[test]
fn an_unreadable_descriptor_is_not_treated_as_absent() {
    let root = Root::new();
    fs::create_dir(root.descriptor()).unwrap();

    assert!(
        matches!(load(root.path()), Err(LoadError::Read { path, .. })
        if path == root.descriptor())
    );
}

#[test]
fn invalid_utf8_is_a_read_error() {
    let root = Root::new();
    fs::write(root.descriptor(), [0xff]).unwrap();

    assert!(
        matches!(load(root.path()), Err(LoadError::Read { path, .. })
        if path == root.descriptor())
    );
}

#[test]
fn parse_errors_keep_the_descriptor_path_and_cause() {
    use std::error::Error;

    let root = Root::new();
    fs::write(
        root.descriptor(),
        "[unit.app]\ndepends_on = [\"missing\"]\n",
    )
    .unwrap();
    let error = load(root.path()).unwrap_err();

    assert!(
        matches!(&error, LoadError::Parse { path, error: ParseError::UnknownDependency { .. } }
        if path == &root.descriptor())
    );
    assert!(
        error
            .to_string()
            .contains(&root.descriptor().display().to_string())
    );
    assert!(error.source().is_some());
}
