use beans_core::{
    classpath::Classpath,
    resource::{ResourceId, ResourceRoot},
};

use super::{classpath, module};
use crate::model::Module;

#[test]
fn own_sources_and_direct_dependencies_are_visible_but_other_modules_are_not() {
    let classpath = classpath(
        Module {
            depends_on: vec!["lib".into()],
            ..module(&["/project/app"])
        },
        &[
            ("lib", module(&["/project/lib"])),
            ("other", module(&["/project/other"])),
        ],
    );
    assert!(classpath.contains_source(&ResourceId::file("/project/app/Future.java")));
    assert!(classpath.contains_source(&ResourceId::file("/project/lib/Future.java")));
    assert!(!classpath.contains_source(&ResourceId::file("/project/other/Future.java")));
}

#[test]
fn dependency_edges_are_not_transitive() {
    let classpath = classpath(
        Module {
            depends_on: vec!["lib".into()],
            ..module(&["/project/app"])
        },
        &[
            (
                "lib",
                Module {
                    depends_on: vec!["core".into()],
                    ..module(&["/project/lib"])
                },
            ),
            ("core", module(&["/project/core"])),
        ],
    );
    assert!(classpath.contains_source(&ResourceId::file("/project/lib/Future.java")));
    assert!(!classpath.contains_source(&ResourceId::file("/project/core/Future.java")));
}

#[test]
fn source_directory_matching_uses_normalized_path_components() {
    let classpath = classpath(module(&["/project/src"]), &[]);
    assert!(classpath.contains_source(&ResourceId::file("/project/src/nested/Future.java")));
    for path in [
        "/project/src-other/Future.java",
        "/project/src/../outside/Future.java",
        "src/Future.java",
    ] {
        assert!(!classpath.contains_source(&ResourceId::file(path)));
    }
}

#[test]
fn source_paths_are_not_uri_encoded() {
    let classpath = classpath(module(&["/project/my sources/é"]), &[]);
    assert!(classpath.contains_source(&ResourceId::file("/project/my sources/é/Future.java")));
}

#[test]
fn virtual_resources_cannot_masquerade_as_local_files() {
    let classpath = classpath(module(&["/project/src"]), &[]);
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
    let classpath = classpath(module(&["/project/src"]), &[]);
    for name in ["Foo.java", "Foo.kt", "config.properties"] {
        assert!(classpath.contains_source(&ResourceId::file(format!("/project/src/{name}"))));
    }
}
