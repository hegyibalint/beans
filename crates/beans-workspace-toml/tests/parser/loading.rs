use std::{fs, path::Path};

use beans_workspace_toml::{LoadError, ParseError, load, parse};

use super::Root;

#[test]
fn an_absent_descriptor_is_not_an_error() {
    let root = Root::new();

    assert!(load(root.path(), None).unwrap().is_none());
}

#[test]
fn a_descriptor_is_resolved_against_its_containing_root() {
    let root = Root::new();
    fs::write(root.descriptor(), "[unit.app]\nsources = [\"src\"]\n").unwrap();

    let workspace = load(root.path(), None).unwrap().unwrap();

    assert_eq!(
        workspace.model().units["app"].sources,
        [root.path().join("src")]
    );
}

#[test]
fn loading_applies_runtime_overrides_without_changing_descriptor_parsing() {
    let root = Root::new();
    let contents = "[unit.app]\n";
    fs::write(root.descriptor(), contents).unwrap();

    let workspace = load(root.path(), Some(Path::new("local jdk")))
        .unwrap()
        .unwrap();
    assert_eq!(
        workspace.model().units["app"].jdk_home,
        Some(root.path().join("local jdk"))
    );
    let parsed = parse(contents, root.path()).unwrap();
    assert_eq!(parsed.model().units["app"].jdk_home, None);
}

#[test]
fn an_unreadable_descriptor_is_not_treated_as_absent() {
    let root = Root::new();
    fs::create_dir(root.descriptor()).unwrap();

    assert!(
        matches!(load(root.path(), None), Err(LoadError::Read { path, .. })
        if path == root.descriptor())
    );
}

#[test]
fn invalid_utf8_is_a_read_error() {
    let root = Root::new();
    fs::write(root.descriptor(), [0xff]).unwrap();

    assert!(
        matches!(load(root.path(), None), Err(LoadError::Read { path, .. })
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
    let error = load(root.path(), None).unwrap_err();

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
