use std::{
    fs,
    path::{Path, PathBuf},
};

use super::artifacts;

#[test]
fn selects_each_supported_runtime_layout() {
    for relative in ["lib/modules", "jre/lib/rt.jar", "lib/rt.jar"] {
        let home = tempfile::tempdir().unwrap();
        let runtime = artifact(home.path(), relative);

        assert_eq!(artifacts(home.path()).unwrap(), vec![runtime]);
    }
}

#[test]
fn runtime_candidates_are_selected_in_order() {
    let home = tempfile::tempdir().unwrap();
    for relative in ["lib/rt.jar", "jre/lib/rt.jar", "lib/modules"] {
        let runtime = artifact(home.path(), relative);

        assert_eq!(artifacts(home.path()).unwrap(), vec![runtime]);
    }
}

#[test]
fn selection_does_not_parse_or_fall_back_from_an_existing_runtime() {
    let home = tempfile::tempdir().unwrap();
    let runtime = artifact(home.path(), "lib/modules");
    fs::write(&runtime, b"invalid image").unwrap();
    artifact(home.path(), "jre/lib/rt.jar");

    assert_eq!(artifacts(home.path()).unwrap(), vec![runtime]);
}

#[test]
fn missing_and_non_file_runtimes_are_errors() {
    let home = tempfile::tempdir().unwrap();
    assert!(
        artifacts(home.path())
            .unwrap_err()
            .contains("no supported JDK runtime")
    );
    fs::create_dir_all(home.path().join("lib/modules")).unwrap();
    artifact(home.path(), "lib/rt.jar");
    assert!(
        artifacts(home.path())
            .unwrap_err()
            .contains("expected a JDK artifact file")
    );
}

#[test]
fn legacy_layouts_include_optional_tools() {
    for relative in ["jre/lib/rt.jar", "lib/rt.jar"] {
        let home = tempfile::tempdir().unwrap();
        let runtime = artifact(home.path(), relative);
        let tools = artifact(home.path(), "lib/tools.jar");

        assert_eq!(artifacts(home.path()).unwrap(), vec![runtime, tools]);
    }
}

#[test]
fn modern_layout_ignores_legacy_artifacts() {
    let home = tempfile::tempdir().unwrap();
    let runtime = artifact(home.path(), "lib/modules");
    artifact(home.path(), "jre/lib/rt.jar");
    artifact(home.path(), "lib/rt.jar");
    fs::create_dir_all(home.path().join("lib/tools.jar")).unwrap();

    assert_eq!(artifacts(home.path()).unwrap(), vec![runtime]);
}

#[test]
fn tools_alone_do_not_supply_a_runtime() {
    let home = tempfile::tempdir().unwrap();
    artifact(home.path(), "lib/tools.jar");

    assert!(
        artifacts(home.path())
            .unwrap_err()
            .contains("no supported JDK runtime")
    );
}

#[test]
fn existing_optional_artifact_must_be_a_file() {
    for relative in ["jre/lib/rt.jar", "lib/rt.jar"] {
        let home = tempfile::tempdir().unwrap();
        artifact(home.path(), relative);
        fs::create_dir_all(home.path().join("lib/tools.jar")).unwrap();

        let error = artifacts(home.path()).unwrap_err();
        assert!(error.contains("lib/tools.jar"));
        assert!(error.contains("expected a JDK artifact file"));
    }
}

fn artifact(home: &Path, relative: &str) -> PathBuf {
    let path = home.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, []).unwrap();
    path
}
