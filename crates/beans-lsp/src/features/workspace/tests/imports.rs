use std::fs;

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
    let features = Features::initialize(&params(&[root.path()])).unwrap();

    assert_eq!(features.workspace_documents.len(), 1);
    let uri = Url::from_file_path(root.path().join("src/Included.java")).unwrap();
    assert!(features.workspace_documents.contains_key(uri.as_str()));
}

#[test]
fn declared_sources_outside_the_project_root_are_indexed() {
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
    let features = Features::initialize(&params(&[&project])).unwrap();

    let uri = Url::from_file_path(shared.join("Included.java")).unwrap();
    assert!(features.workspace_documents.contains_key(uri.as_str()));
}

#[test]
fn an_absent_descriptor_keeps_fallback_ingestion() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Example.java"), "").unwrap();
    let features = Features::initialize(&params(&[root.path()])).unwrap();

    assert_eq!(features.workspace_documents.len(), 1);
    let uri = Url::from_file_path(root.path().join("Example.java")).unwrap();
    assert!(features.workspace_documents.contains_key(uri.as_str()));
}

#[test]
fn a_bad_descriptor_prevents_cache_creation() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Example.java"), "").unwrap();
    fs::write(
        root.path().join("beans.toml"),
        "[unit.main]\ndepends_on = [\"missing\"]\n",
    )
    .unwrap();

    let error = Features::initialize(&params(&[root.path()])).err().unwrap();

    assert!(error.contains(&root.path().join("beans.toml").display().to_string()));
    assert!(!root.path().join(".beans").exists());
}

#[test]
fn configured_binary_inputs_are_not_ingested_as_source_text() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("beans.toml"),
        "[unit.main]\nclasspath = [\"lib.jar\"]\njdk_home = \"jdk\"\n",
    )
    .unwrap();
    let features = Features::initialize(&params(&[root.path()])).unwrap();

    assert!(features.workspace_documents.is_empty());
}
