use std::{fs, path::Path};

use beans_testing::TempDir;
use beans_workspace_toml::{DESCRIPTOR, LoadError, ParseError, load, parse};

#[test]
fn an_absent_descriptor_is_not_an_error() {
    let files = TempDir::new();

    assert!(load(files.root(), None).unwrap().is_none());
}

#[test]
fn a_descriptor_is_resolved_against_its_containing_root() {
    let files = TempDir::new();
    files.write(DESCRIPTOR, "[unit.app]\nsources = [\"src\"]\n");

    let workspace = load(files.root(), None).unwrap().unwrap();

    assert_eq!(workspace.model().units["app"].sources, [files.path("src")]);
}

#[test]
fn loading_applies_runtime_overrides_without_changing_descriptor_parsing() {
    let files = TempDir::new();
    let contents = "[unit.app]\n";
    files.write(DESCRIPTOR, contents);

    let workspace = load(files.root(), Some(Path::new("local jdk")))
        .unwrap()
        .unwrap();
    assert_eq!(
        workspace.model().units["app"].jdk_home,
        Some(files.path("local jdk"))
    );
    let parsed = parse(contents, files.root()).unwrap();
    assert_eq!(parsed.model().units["app"].jdk_home, None);
}

#[test]
fn an_unreadable_descriptor_is_not_treated_as_absent() {
    let files = TempDir::new();
    let descriptor = files.path(DESCRIPTOR);
    fs::create_dir(&descriptor).unwrap();

    assert!(
        matches!(load(files.root(), None), Err(LoadError::Read { path, .. })
        if path == descriptor)
    );
}

#[test]
fn invalid_utf8_is_a_read_error() {
    let files = TempDir::new();
    let descriptor = files.write(DESCRIPTOR, [0xff]);

    assert!(
        matches!(load(files.root(), None), Err(LoadError::Read { path, .. })
        if path == descriptor)
    );
}

#[test]
fn parse_errors_keep_the_descriptor_path_and_cause() {
    use std::error::Error;

    let files = TempDir::new();
    let descriptor = files.write(DESCRIPTOR, "[unit.app]\ndepends_on = [\"missing\"]\n");
    let error = load(files.root(), None).unwrap_err();

    assert!(
        matches!(&error, LoadError::Parse { path, error: ParseError::UnknownDependency { .. } }
        if path == &descriptor)
    );
    assert!(
        error
            .to_string()
            .contains(&descriptor.display().to_string())
    );
    assert!(error.source().is_some());
}
