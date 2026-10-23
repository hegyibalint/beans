use std::path::Path;

use beans_core::location::{Location, LocationEntry};
use beans_workspace::Workspace;

use beans_workspace_toml::{TomlWorkspace, parse};

fn entry(path: &str, name: &str) -> Location {
    Location::new(LocationEntry::File(path.into())).entry(LocationEntry::Container(name.into()))
}

fn workspace(contents: &str) -> TomlWorkspace {
    parse(contents, Path::new("/project")).unwrap()
}

#[test]
fn each_document_selects_its_own_units_policy() {
    let workspace = workspace(
        r#"
        [unit.app]
        sources = ["app"]
        classpath = ["app.jar"]
        [unit.lib]
        sources = ["lib"]
        classpath = ["lib.jar"]
        "#,
    );

    for (document, visible, hidden) in [
        (
            "file:///project/app/Future.java",
            "/project/app.jar",
            "/project/lib.jar",
        ),
        (
            "file:///project/lib/Future.java",
            "/project/lib.jar",
            "/project/app.jar",
        ),
    ] {
        let classpath = workspace.classpath_for(document);
        assert!(classpath.contains_class(&entry(visible, "Example.class")));
        assert!(!classpath.contains_class(&entry(hidden, "Example.class")));
    }
}

#[test]
fn shared_roots_combine_the_policies_of_all_owners() {
    let workspace = workspace(
        r#"
        [unit.app]
        sources = ["shared"]
        classpath = ["app.jar"]
        [unit.lib]
        sources = ["shared"]
        classpath = ["lib.jar"]
        "#,
    );
    let classpath = workspace.classpath_for("file:///project/shared/Future.java");

    assert!(classpath.contains_class(&entry("/project/app.jar", "Example.class")));
    assert!(classpath.contains_class(&entry("/project/lib.jar", "Example.class")));
    assert!(!classpath.contains_class(&entry("/project/other.jar", "Example.class")));
}

#[test]
fn nested_roots_add_owners_only_within_the_nested_tree() {
    let workspace = workspace(
        r#"
        [unit.broad]
        sources = ["src"]
        classpath = ["broad.jar"]
        [unit.nested]
        sources = ["src/nested"]
        classpath = ["nested.jar"]
        "#,
    );
    let nested = workspace.classpath_for("file:///project/src/nested/Future.java");
    let outer = workspace.classpath_for("file:///project/src/Future.java");

    assert!(nested.contains_class(&entry("/project/broad.jar", "Example.class")));
    assert!(nested.contains_class(&entry("/project/nested.jar", "Example.class")));
    assert!(outer.contains_class(&entry("/project/broad.jar", "Example.class")));
    assert!(!outer.contains_class(&entry("/project/nested.jar", "Example.class")));
}

#[test]
fn unclaimed_and_non_file_documents_remain_unrestricted() {
    let workspace = workspace("[unit.app]\nsources = ['src']\nclasspath = ['app.jar']\n");

    for uri in [
        "file:///project/scratch/Future.java",
        "file:///project/src-other/Future.java",
        "untitled:Future.java",
        "beans-jvm:///library/Example.class",
        "beans-jvm:///project/src/Future.java",
        "custom:///project/src/Future.java",
        "not a URI",
    ] {
        assert!(
            workspace
                .classpath_for(uri)
                .contains_class(&entry("/unrelated.jar", "Example.class")),
            "{uri}"
        );
    }
}

#[test]
fn file_uris_are_decoded_and_normalized_before_ownership_is_selected() {
    let workspace = workspace("[unit.app]\nsources = ['my sources/é']\nclasspath = ['app.jar']\n");
    let classpath =
        workspace.classpath_for("file:///project/old/../my%20sources/%C3%A9/Future.java");

    assert!(classpath.contains_class(&entry("/project/app.jar", "Example.class")));
    assert!(!classpath.contains_class(&entry("/unrelated.jar", "Example.class")));
}
