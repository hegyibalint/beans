use std::path::{Path, PathBuf};

use beans_testing::TempDir;
use beans_workspace_toml::{DESCRIPTOR, LoadError, ParseError, load};

const CONTENTS: &str = r#"
jdk_home = "vendor/jdk"
[unit.app]
sources = ["app"]
[unit.legacy]
sources = ["legacy"]
jdk_home = "vendor/legacy"
"#;

#[test]
fn an_override_supplies_a_missing_jdk() {
    let files = TempDir::new();
    files.write(DESCRIPTOR, "[unit.app]\n");

    let workspace = load(files.root(), Some(Path::new("/local/jdk")))
        .unwrap()
        .unwrap();

    assert_eq!(
        workspace.model().units["app"].jdk_home,
        Some(PathBuf::from("/local/jdk"))
    );
}

#[test]
fn an_override_wins_over_both_workspace_and_unit_jdks() {
    let files = TempDir::new();
    files.write(DESCRIPTOR, CONTENTS);
    let home = Path::new("/local/jdk");

    let workspace = load(files.root(), Some(home)).unwrap().unwrap();

    for unit in workspace.model().units.values() {
        assert_eq!(unit.jdk_home.as_deref(), Some(home));
    }
}

#[test]
fn relative_overrides_are_normalized_against_the_workspace_root() {
    let files = TempDir::new();
    files.write(DESCRIPTOR, "[unit.app]\n");

    let workspace = load(files.root(), Some(Path::new("local/.././jdk with spaces")))
        .unwrap()
        .unwrap();

    assert_eq!(
        workspace.model().units["app"].jdk_home,
        Some(files.path("jdk with spaces"))
    );
}

#[test]
fn absent_and_empty_overrides_preserve_descriptor_precedence() {
    let files = TempDir::new();
    files.write(DESCRIPTOR, CONTENTS);

    for jdk_override in [None, Some(Path::new(""))] {
        let workspace = load(files.root(), jdk_override).unwrap().unwrap();

        assert_eq!(
            workspace.model().units["app"].jdk_home,
            Some(files.path("vendor/jdk"))
        );
        assert_eq!(
            workspace.model().units["legacy"].jdk_home,
            Some(files.path("vendor/legacy"))
        );
    }
}

#[test]
fn an_override_does_not_hide_an_invalid_descriptor() {
    let files = TempDir::new();
    files.write(DESCRIPTOR, "jdk_home = 26\n[unit.app]\n");

    assert!(matches!(
        load(files.root(), Some(Path::new("/local/jdk"))),
        Err(LoadError::Parse {
            error: ParseError::Toml(_),
            ..
        })
    ));
}
