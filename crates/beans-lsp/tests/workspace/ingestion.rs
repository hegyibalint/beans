use std::path::Path;

use beans_core::location::Location;

use super::*;

fn stored_sources(root: &Path) -> Vec<Location> {
    let connection = beans_engine::storage::open(&root.join(".beans/models")).unwrap();
    let mut statement = connection
        .prepare("SELECT location FROM models WHERE kind = 1 ORDER BY location")
        .unwrap();
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .map(|row| serde_json::from_str(&row.unwrap()).unwrap())
        .collect()
}

#[test]
fn a_descriptor_limits_ingestion_to_declared_sources_and_deduplicates_them() {
    let root = TempDir::new();
    let included = root.write("src/Included.java", "class Included {}");
    root.write("Excluded.java", "class Excluded {}");
    root.write(
        "beans.toml",
        "[unit.main]\nsources = ['src']\n[unit.shared]\nsources = ['src']\n",
    );
    let mut client = Client::default();
    assert!(
        request(&mut client, "initialize", initialize_params(root.root()))
            .error
            .is_none()
    );
    drop(client);

    assert_eq!(stored_sources(root.root()), [Location::file(included)]);
}

#[test]
fn declared_sources_outside_the_project_root_are_indexed() {
    let root = TempDir::new();
    let included = root.write("shared/Included.java", "class Included {}");
    root.write(
        "project/beans.toml",
        "[unit.main]\nsources = ['../shared']\n",
    );
    let project = root.path("project");
    let mut client = Client::default();
    assert!(
        request(&mut client, "initialize", initialize_params(&project))
            .error
            .is_none()
    );
    drop(client);

    assert_eq!(stored_sources(&project), [Location::file(included)]);
}

#[test]
fn an_absent_descriptor_keeps_fallback_ingestion() {
    let root = TempDir::new();
    let included = root.write("Example.java", "class Example {}");
    let mut client = Client::default();
    assert!(
        request(&mut client, "initialize", initialize_params(root.root()))
            .error
            .is_none()
    );
    drop(client);

    assert_eq!(stored_sources(root.root()), [Location::file(included)]);
}

#[test]
fn configured_binary_inputs_are_not_ingested_as_source_text() {
    let root = TempDir::new();
    root.write(
        "beans.toml",
        "[unit.main]\nclasspath = ['lib.jar']\njdk_home = 'jdk'\n",
    );
    root.write(
        "lib.jar",
        beans_testing::support::jar_bytes(&[(
            "p/Library.class",
            &beans_testing::support::class_bytes("p/Library"),
        )]),
    );
    root.write(
        "jdk/lib/modules",
        beans_testing::support::class_bytes("p/Runtime"),
    );
    let mut client = Client::default();
    assert!(
        request(&mut client, "initialize", initialize_params(root.root()))
            .error
            .is_none()
    );
    drop(client);

    assert!(stored_sources(root.root()).is_empty());
}
