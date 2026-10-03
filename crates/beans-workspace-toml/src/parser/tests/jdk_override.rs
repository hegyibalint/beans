use std::path::{Path, PathBuf};

use crate::parser::{ParseError, descriptor::parse_with_jdk_override};

const DESCRIPTOR: &str =
    "jdk_home = \"vendor/jdk\"\n[unit.app]\n[unit.legacy]\njdk_home = \"vendor/legacy\"\n";

#[test]
fn an_override_supplies_a_missing_jdk() {
    let workspace = parse_with_jdk_override(
        "[unit.app]\n",
        Path::new("/project"),
        Some(Path::new("/local/jdk")),
    )
    .unwrap();

    assert_eq!(
        workspace.model().units["app"].jdk_home,
        Some(PathBuf::from("/local/jdk"))
    );
}

#[test]
fn an_override_wins_over_both_workspace_and_unit_jdks() {
    let workspace = parse_with_jdk_override(
        DESCRIPTOR,
        Path::new("/project"),
        Some(Path::new("/local/jdk")),
    )
    .unwrap();

    for unit in workspace.model().units.values() {
        assert_eq!(unit.jdk_home, Some(PathBuf::from("/local/jdk")));
    }
}

#[test]
fn relative_overrides_are_normalized_against_the_workspace_root() {
    let workspace = parse_with_jdk_override(
        "[unit.app]\n",
        Path::new("/project"),
        Some(Path::new("../local/./jdk with spaces")),
    )
    .unwrap();

    assert_eq!(
        workspace.model().units["app"].jdk_home,
        Some(PathBuf::from("/local/jdk with spaces"))
    );
}

#[test]
fn absent_and_empty_overrides_preserve_descriptor_precedence() {
    for jdk_override in [None, Some(Path::new(""))] {
        let workspace =
            parse_with_jdk_override(DESCRIPTOR, Path::new("/project"), jdk_override).unwrap();

        assert_eq!(
            workspace.model().units["app"].jdk_home,
            Some(PathBuf::from("/project/vendor/jdk"))
        );
        assert_eq!(
            workspace.model().units["legacy"].jdk_home,
            Some(PathBuf::from("/project/vendor/legacy"))
        );
    }
}

#[test]
fn an_override_does_not_hide_an_invalid_descriptor() {
    assert!(matches!(
        parse_with_jdk_override(
            "jdk_home = 26\n[unit.app]\n",
            Path::new("/project"),
            Some(Path::new("/local/jdk")),
        ),
        Err(ParseError::Toml(_))
    ));
}
