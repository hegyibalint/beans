use beans_core::{
    classpath::Classpath,
    resource::{ResourceId, ResourceRoot},
};

use super::{classpath, unit};

fn file(path: &str) -> ResourceId {
    ResourceId::new(ResourceRoot::File { path: path.into() })
}

#[test]
fn own_sources_and_direct_dependencies_are_visible_but_other_units_are_not() {
    let classpath = classpath(
        crate::model::Unit {
            depends_on: vec!["lib".into()],
            ..unit(&["/project/app"])
        },
        &[
            ("lib", unit(&["/project/lib"])),
            ("other", unit(&["/project/other"])),
        ],
    );
    assert!(classpath.contains_source(&file("/project/app/Future.java")));
    assert!(classpath.contains_source(&file("/project/lib/Future.java")));
    assert!(!classpath.contains_source(&file("/project/other/Future.java")));
}

#[test]
fn dependency_edges_are_not_transitive() {
    let classpath = classpath(
        crate::model::Unit {
            depends_on: vec!["lib".into()],
            ..unit(&["/project/app"])
        },
        &[
            (
                "lib",
                crate::model::Unit {
                    depends_on: vec!["core".into()],
                    ..unit(&["/project/lib"])
                },
            ),
            ("core", unit(&["/project/core"])),
        ],
    );
    assert!(classpath.contains_source(&file("/project/lib/Future.java")));
    assert!(!classpath.contains_source(&file("/project/core/Future.java")));
}

#[test]
fn source_directory_matching_uses_normalized_path_components() {
    let classpath = classpath(unit(&["/project/src"]), &[]);
    assert!(classpath.contains_source(&file("/project/src/nested/Future.java")));
    assert!(!classpath.contains_source(&file("/project/src-other/Future.java")));
    assert!(!classpath.contains_source(&file("/project/src/../outside/Future.java")));
    assert!(!classpath.contains_source(&file("src/Future.java")));
}

#[test]
fn source_paths_are_not_uri_encoded() {
    let classpath = classpath(unit(&["/project/my sources/é"]), &[]);
    assert!(classpath.contains_source(&file("/project/my sources/é/Future.java")));
}

#[test]
fn virtual_resources_cannot_masquerade_as_local_files() {
    let classpath = classpath(unit(&["/project/src"]), &[]);
    for provider in ["untitled", "beans-jvm", "file"] {
        let resource = ResourceId::new(ResourceRoot::Virtual {
            provider: provider.into(),
            key: "/project/src/Future.java".into(),
        });
        assert!(!classpath.contains_source(&resource));
        assert!(!classpath.contains_class(&resource));
    }
}

#[test]
fn source_visibility_is_independent_of_file_format() {
    let classpath = classpath(unit(&["/project/src"]), &[]);
    for name in ["Foo.java", "Foo.kt", "config.properties"] {
        assert!(classpath.contains_source(&file(&format!("/project/src/{name}"))));
    }
}
