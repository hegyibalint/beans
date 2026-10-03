use std::fs;

use beans_core::{origin::Origin, revision::Revision};
use url::Url;

use crate::features::Features;

use super::params;

#[test]
fn a_descriptor_limits_ingestion_to_declared_sources() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("src")).unwrap();
    fs::write(root.path().join("src/Included.java"), "").unwrap();
    fs::write(root.path().join("Excluded.java"), "").unwrap();
    fs::write(
        root.path().join("beans.toml"),
        r#"
        [unit.main]
        sources = ["src"]
        [unit.shared]
        sources = ["src"]
    "#,
    )
    .unwrap();
    let mut features = Features::default();

    features.initialize(&params(&[root.path()])).unwrap();

    assert_eq!(features.workspace_documents.len(), 1);
    assert_eq!(features.languages.revision(), Revision::new(1));
    let uri = Url::from_file_path(root.path().join("src/Included.java")).unwrap();
    assert!(features.workspace_documents.contains_key(uri.as_str()));
}

#[test]
fn declared_sources_outside_the_project_root_are_indexed_and_scoped() {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("project");
    let shared = root.path().join("shared");
    fs::create_dir(&project).unwrap();
    fs::create_dir(&shared).unwrap();
    fs::write(shared.join("Included.java"), "").unwrap();
    fs::write(
        project.join("beans.toml"),
        "[unit.main]\nsources = [\"../shared\"]\n",
    )
    .unwrap();
    let mut features = Features::default();

    features.initialize(&params(&[&project])).unwrap();

    let uri = Url::from_file_path(shared.join("Included.java")).unwrap();
    assert!(features.workspace_documents.contains_key(uri.as_str()));
    assert!(
        !features
            .workspaces
            .classpath_for(uri.as_str())
            .contains(&Origin::jar_entry("/unrelated.jar", "Example.class"))
    );
}

#[test]
fn an_absent_descriptor_keeps_fallback_ingestion_and_unrestricted_visibility() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Example.java"), "").unwrap();
    let mut features = Features::default();

    features.initialize(&params(&[root.path()])).unwrap();

    assert_eq!(features.workspace_documents.len(), 1);
    let uri = Url::from_file_path(root.path().join("Example.java")).unwrap();
    assert!(
        features
            .workspaces
            .classpath_for(uri.as_str())
            .contains(&Origin::jar_entry("/unrelated.jar", "Example.class"))
    );
}

#[test]
fn a_bad_descriptor_prevents_partial_imports_in_other_folders() {
    let root = tempfile::tempdir().unwrap();
    let valid = root.path().join("a-valid");
    let invalid = root.path().join("b-invalid");
    fs::create_dir(&valid).unwrap();
    fs::create_dir(&invalid).unwrap();
    fs::write(valid.join("Example.java"), "").unwrap();
    fs::write(valid.join("beans.toml"), "[unit.main]\nsources = [\".\"]\n").unwrap();
    fs::write(
        invalid.join("beans.toml"),
        "[unit.main]\ndepends_on = [\"missing\"]\n",
    )
    .unwrap();
    let mut features = Features::default();

    let error = features
        .initialize(&params(&[&valid, &invalid]))
        .unwrap_err();

    assert!(
        error
            .to_string()
            .contains(&invalid.join("beans.toml").display().to_string())
    );
    assert!(features.workspace_documents.is_empty());
    assert_eq!(features.languages.revision(), Revision::default());
}

#[test]
fn configured_binary_inputs_are_not_ingested_as_source_text() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("beans.toml"),
        "[unit.main]\nclasspath = [\"lib.jar\"]\njdk_home = \"jdk\"\n",
    )
    .unwrap();
    let mut features = Features::default();

    features.initialize(&params(&[root.path()])).unwrap();
    assert!(features.workspace_documents.is_empty());
}
