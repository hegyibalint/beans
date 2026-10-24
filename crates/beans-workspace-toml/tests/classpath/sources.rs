use beans_core::location::{Location, LocationEntry};
use beans_workspace::Workspace;

use super::workspace;

fn file(path: &str) -> Location {
    Location::new(LocationEntry::File(path.into()))
}

#[test]
fn own_sources_and_direct_dependencies_are_visible_but_other_units_are_not() {
    let workspace = workspace(
        r#"
        [unit.app]
        sources = ["app"]
        depends_on = ["lib"]
        [unit.lib]
        sources = ["lib"]
        [unit.other]
        sources = ["other"]
        "#,
    );
    let classpath = workspace.classpath_for("file:///project/app/Use.java");
    assert!(classpath.contains_source(&file("/project/app/Future.java")));
    assert!(classpath.contains_source(&file("/project/lib/Future.java")));
    assert!(!classpath.contains_source(&file("/project/other/Future.java")));
}

#[test]
fn dependency_edges_are_not_transitive() {
    let workspace = workspace(
        r#"
        [unit.app]
        sources = ["app"]
        depends_on = ["lib"]
        [unit.lib]
        sources = ["lib"]
        depends_on = ["core"]
        [unit.core]
        sources = ["core"]
        "#,
    );
    let classpath = workspace.classpath_for("file:///project/app/Use.java");
    assert!(classpath.contains_source(&file("/project/lib/Future.java")));
    assert!(!classpath.contains_source(&file("/project/core/Future.java")));
}

#[test]
fn source_directory_matching_uses_normalized_path_components() {
    let workspace = workspace("[unit.app]\nsources = ['src']\n");
    let classpath = workspace.classpath_for("file:///project/src/Use.java");
    assert!(classpath.contains_source(&file("/project/src/nested/Future.java")));
    assert!(!classpath.contains_source(&file("/project/src-other/Future.java")));
    assert!(!classpath.contains_source(&file("/project/src/../outside/Future.java")));
    assert!(!classpath.contains_source(&file("src/Future.java")));
}

#[test]
fn source_paths_are_not_uri_encoded() {
    let workspace = workspace("[unit.app]\nsources = ['my sources/é']\n");
    let classpath = workspace.classpath_for("file:///project/my%20sources/%C3%A9/Use.java");
    assert!(classpath.contains_source(&file("/project/my sources/é/Future.java")));
}

#[test]
fn virtual_resources_cannot_masquerade_as_local_files() {
    let workspace = workspace("[unit.app]\nsources = ['src']\n");
    let classpath = workspace.classpath_for("file:///project/src/Use.java");
    for scheme in ["untitled", "beans-jvm", "file"] {
        let resource = Location::new(LocationEntry::Virtual(format!(
            "{scheme}:///project/src/Future.java"
        )));
        assert!(!classpath.contains_source(&resource));
        assert!(!classpath.contains_class(&resource));
    }
}

#[test]
fn source_visibility_is_independent_of_file_format() {
    let workspace = workspace("[unit.app]\nsources = ['src']\n");
    let classpath = workspace.classpath_for("file:///project/src/Use.java");
    for name in ["Foo.java", "Foo.kt", "config.properties"] {
        assert!(classpath.contains_source(&file(&format!("/project/src/{name}"))));
    }
}
