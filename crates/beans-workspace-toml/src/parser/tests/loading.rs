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
fn loading_reads_the_environment_but_parsing_does_not() {
    const CHILD_ROOT: &str = "BEANS_TEST_WORKSPACE_ROOT";
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        let root = PathBuf::from(root);
        let workspace = load(&root).unwrap().unwrap();
        assert_eq!(
            workspace.model().units["app"].jdk_home,
            Some(root.join("local jdk"))
        );
        let parsed = crate::parser::parse("[unit.app]\n", &root).unwrap();
        assert_eq!(parsed.model().units["app"].jdk_home, None);
        return;
    }

    let root = Root::new();
    fs::write(root.descriptor(), "[unit.app]\n").unwrap();
    // A child process keeps environment changes isolated from parallel tests.
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "parser::tests::loading::loading_reads_the_environment_but_parsing_does_not",
            "--nocapture",
        ])
        .env(CHILD_ROOT, root.path())
        .env(
            "BEANS_WORKSPACE_TOML_JAVA_HOME",
            root.path().join("local jdk"),
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child test failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
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
