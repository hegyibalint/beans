use beans_core::location::{Location, LocationEntry};
use beans_workspace::Workspace;

use super::workspace;

fn file(path: &str) -> Location {
    Location::new(LocationEntry::File(path.into()))
}

#[test]
fn source_roots_and_binary_locations_have_independent_visibility() {
    let workspace = workspace("[unit.app]\nsources = ['src']\nclasspath = ['out']\n");
    let classpath = workspace.classpath_for("file:///project/src/Use.java");

    for name in ["Foo.java", "Foo.class"] {
        let source = file(&format!("/project/src/{name}"));
        let binary = file(&format!("/project/out/{name}"));
        assert!(classpath.contains_source(&source));
        assert!(!classpath.contains_class(&source));
        assert!(!classpath.contains_source(&binary));
        assert!(classpath.contains_class(&binary));
    }
}

#[test]
fn a_location_can_be_explicitly_included_in_both_policies() {
    let workspace = workspace("[unit.app]\nsources = ['shared']\nclasspath = ['shared']\n");
    let classpath = workspace.classpath_for("file:///project/shared/Use.java");
    let resource = file("/project/shared/Foo.java");
    assert!(classpath.contains_source(&resource));
    assert!(classpath.contains_class(&resource));
}

#[test]
fn source_entries_in_archives_are_not_supported_even_under_source_roots() {
    let workspace = workspace("[unit.app]\nsources = ['src']\nclasspath = ['src/lib.jar']\n");
    let classpath = workspace.classpath_for("file:///project/src/Use.java");
    let archive = file("/project/src/lib.jar");
    for container in [
        archive.clone(),
        archive.entry(LocationEntry::Container("nested.jar".into())),
    ] {
        let source = container.entry(LocationEntry::Container("p/Foo.java".into()));
        let binary = container.entry(LocationEntry::Container("p/Foo.class".into()));
        assert!(!classpath.contains_source(&source));
        assert!(!classpath.contains_source(&binary));
        assert!(classpath.contains_class(&binary));
    }
}
