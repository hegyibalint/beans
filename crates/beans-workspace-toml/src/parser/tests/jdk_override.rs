use std::path::{Path, PathBuf};

use crate::parser::{ParseError, descriptor::parse_with_jdk_override};

const DESCRIPTOR: &str = "jdk_home = \"vendor/jdk\"\n[module.app]\n[module.test]\n";

#[test]
fn an_override_supplies_a_missing_jdk() {
    let workspace = parse_with_jdk_override(
        "[module.app]\n",
        Path::new("/project"),
        Some(Path::new("/local/jdk")),
    )
    .unwrap();

    assert_eq!(
        workspace.model().jdk_home,
        Some(PathBuf::from("/local/jdk"))
    );
}

#[test]
fn an_override_wins_over_the_project_jdk() {
    let workspace = parse_with_jdk_override(
        DESCRIPTOR,
        Path::new("/project"),
        Some(Path::new("/local/jdk")),
    )
    .unwrap();

    assert_eq!(
        workspace.model().jdk_home,
        Some(PathBuf::from("/local/jdk"))
    );
}

#[test]
fn relative_overrides_are_normalized_against_the_workspace_root() {
    let workspace = parse_with_jdk_override(
        "[module.app]\n",
        Path::new("/project"),
        Some(Path::new("../local/./jdk with spaces")),
    )
    .unwrap();

    assert_eq!(
        workspace.model().jdk_home,
        Some(PathBuf::from("/local/jdk with spaces"))
    );
}

#[test]
fn absent_and_empty_overrides_preserve_the_project_jdk() {
    for jdk_override in [None, Some(Path::new(""))] {
        let workspace =
            parse_with_jdk_override(DESCRIPTOR, Path::new("/project"), jdk_override).unwrap();

        assert_eq!(
            workspace.model().jdk_home,
            Some(PathBuf::from("/project/vendor/jdk"))
        );
    }
}

#[test]
fn an_override_does_not_hide_an_invalid_descriptor() {
    assert!(matches!(
        parse_with_jdk_override(
            "jdk_home = 26\n[module.app]\n",
            Path::new("/project"),
            Some(Path::new("/local/jdk"))
        ),
        Err(ParseError::Toml(_))
    ));
}
