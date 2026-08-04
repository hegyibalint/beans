// Listing what a workspace declares, before anything is read or scoped.

use std::path::Path;

use super::*;

/// This crate's own tree stands in for a project: it holds nested
/// directories and not one `.java` file.
fn rust_crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

#[test]
fn a_tree_that_does_not_exist_contributes_nothing() {
    let root = rust_crate_root().join("no-such-directory");
    let workspace = workspace(vec![unit(
        "unit",
        vec![tree(root.to_str().expect("utf-8 path"))],
    )]);

    assert!(java_sources(&workspace).is_empty());
}

#[test]
fn a_tree_contributes_only_java_files() {
    let root = rust_crate_root();
    let workspace = workspace(vec![unit(
        "unit",
        vec![tree(root.to_str().expect("utf-8 path"))],
    )]);

    assert!(java_sources(&workspace).is_empty(), "this tree is all Rust");
}

#[test]
fn classpath_elements_keep_their_unit_order() {
    let workspace = workspace(vec![
        Unit {
            classpath: vec![PathBuf::from("a.jar"), PathBuf::from("b.jar")],
            ..unit("a", Vec::new())
        },
        Unit {
            classpath: vec![PathBuf::from("c.jar")],
            ..unit("b", Vec::new())
        },
    ]);

    assert_eq!(
        classpath(&workspace),
        [
            PathBuf::from("a.jar"),
            PathBuf::from("b.jar"),
            PathBuf::from("c.jar"),
        ]
    );
}

#[test]
fn listed_files_are_filtered_by_extension() {
    let workspace = workspace(vec![unit(
        "unit",
        vec![Selector::Files {
            files: vec![PathBuf::from("A.java"), PathBuf::from("build.gradle.kts")],
            generated: false,
        }],
    )]);

    assert_eq!(java_sources(&workspace), [PathBuf::from("A.java")]);
}
