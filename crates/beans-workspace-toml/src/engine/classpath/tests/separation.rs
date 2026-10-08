use beans_core::{
    classpath::Classpath,
    resource::{ResourceEntry, ResourceId, ResourceRoot},
};

use super::{classpath, unit};
use crate::model::Unit;

fn file(path: &str) -> ResourceId {
    ResourceId::new(ResourceRoot::File { path: path.into() })
}

#[test]
fn source_roots_and_binary_locations_have_independent_visibility() {
    let classpath = classpath(
        Unit {
            classpath: vec!["/project/out".into()],
            ..unit(&["/project/src"])
        },
        &[],
    );

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
    let classpath = classpath(
        Unit {
            classpath: vec!["/project/shared".into()],
            ..unit(&["/project/shared"])
        },
        &[],
    );
    let resource = file("/project/shared/Foo.java");
    assert!(classpath.contains_source(&resource));
    assert!(classpath.contains_class(&resource));
}

#[test]
fn source_entries_in_archives_are_not_supported_even_under_source_roots() {
    let classpath = classpath(
        Unit {
            classpath: vec!["/project/src/lib.jar".into()],
            ..unit(&["/project/src"])
        },
        &[],
    );
    let archive = file("/project/src/lib.jar");
    for container in [
        archive.clone(),
        archive.entry(ResourceEntry("nested.jar".into())),
    ] {
        let source = container.entry(ResourceEntry("p/Foo.java".into()));
        let binary = container.entry(ResourceEntry("p/Foo.class".into()));
        assert!(!classpath.contains_source(&source));
        assert!(!classpath.contains_source(&binary));
        assert!(classpath.contains_class(&binary));
    }
}
