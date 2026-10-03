use beans_core::{classpath::Classpath, origin::Origin};

use super::{classpath, unit};

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

    assert!(classpath.contains(&Origin::uri("file:///project/app/Future.java")));
    assert!(classpath.contains(&Origin::uri("file:///project/lib/Future.java")));
    assert!(!classpath.contains(&Origin::uri("file:///project/other/Future.java")));
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

    assert!(classpath.contains(&Origin::uri("file:///project/lib/Future.java")));
    assert!(!classpath.contains(&Origin::uri("file:///project/core/Future.java")));
}

#[test]
fn source_directory_matching_uses_path_components_not_text_prefixes() {
    let classpath = classpath(unit(&["/project/src"]), &[]);

    assert!(classpath.contains(&Origin::uri("file:///project/src/nested/Future.java")));
    assert!(!classpath.contains(&Origin::uri("file:///project/src-other/Future.java")));
    assert!(!classpath.contains(&Origin::uri("file:///project/src/../outside/Future.java")));
}

#[test]
fn encoded_file_uris_use_filesystem_path_identity() {
    let classpath = classpath(unit(&["/project/my sources/é"]), &[]);

    assert!(classpath.contains(&Origin::uri(
        "file:///project/my%20sources/%C3%A9/Future.java"
    )));
}

#[test]
fn non_file_and_invalid_uris_are_not_in_a_scoped_classpath() {
    let classpath = classpath(unit(&["/project/src"]), &[]);

    for uri in [
        "untitled:Future.java",
        "beans-jvm:///library/Example.class",
        "beans-jvm:///project/src/Future.java",
        "not a URI",
    ] {
        assert!(!classpath.contains(&Origin::uri(uri)), "{uri}");
    }
}
