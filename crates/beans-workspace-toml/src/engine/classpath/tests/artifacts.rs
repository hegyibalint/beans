use beans_core::{
    classpath::Classpath,
    resource::{ResourceEntry, ResourceId, ResourceRoot},
};

use super::{classpath, unit};
use crate::model::Unit;

fn file(path: &str) -> ResourceId {
    ResourceId::new(ResourceRoot::File { path: path.into() })
}

fn entry(path: &str, name: &str) -> ResourceId {
    file(path).entry(ResourceEntry(name.into()))
}

#[test]
fn classpath_entries_admit_standalone_classes_and_classes_in_output_directories() {
    let classpath = classpath(
        Unit {
            classpath: vec!["/project/out".into(), "/project/Feature.class".into()],
            ..unit(&[])
        },
        &[],
    );
    for path in [
        "/project/Feature.class",
        "/project/out/p/Feature.class",
        "/project/out/p/../Feature.class",
    ] {
        assert!(classpath.contains_class(&file(path)));
    }
    for path in [
        "/project/Other.class",
        "/project/out-other/Feature.class",
        "out/Feature.class",
    ] {
        assert!(!classpath.contains_class(&file(path)));
    }
}

#[test]
fn archive_entries_are_matched_by_their_container_not_entry_name() {
    let classpath = classpath(
        Unit {
            classpath: vec![
                "/project/lib.jar".into(),
                "/project/lib.jmod".into(),
                "/project/modules".into(),
            ],
            ..unit(&[])
        },
        &[],
    );
    for (path, name) in [
        ("/project/lib.jar", "p/Feature.class"),
        ("/project/lib.jmod", "classes/p/Feature.class"),
        ("/project/modules", "java.base/p/Feature.class"),
    ] {
        assert!(classpath.contains_class(&entry(path, name)));
    }
    for path in [
        "/project/other.jar",
        "/project/other.jmod",
        "/project/other-modules",
        "lib.jar",
    ] {
        assert!(!classpath.contains_class(&entry(path, "p/Feature.class")));
    }
}

#[test]
fn directories_do_not_implicitly_include_archives_inside_them() {
    let classpath = classpath(
        Unit {
            classpath: vec!["/project/lib".into()],
            ..unit(&["/project/src"])
        },
        &[],
    );
    for path in ["/project/lib/dependency.jar", "/project/src/dependency.jar"] {
        assert!(!classpath.contains_class(&entry(path, "p/Feature.class")));
    }
}

#[test]
fn nested_entries_follow_the_explicitly_included_outer_artifact() {
    let classpath = classpath(
        Unit {
            classpath: vec!["/project/app.zip".into()],
            ..unit(&[])
        },
        &[],
    );
    let nested =
        entry("/project/app.zip", "lib/a.jar").entry(ResourceEntry("p/Feature.class".into()));
    let other = entry("/other/app.zip", "lib/a.jar").entry(ResourceEntry("p/Feature.class".into()));
    assert!(classpath.contains_class(&nested));
    assert!(!classpath.contains_class(&other));
}

#[test]
fn a_jdk_contributes_only_its_own_runtime_image() {
    let classpath = classpath(
        Unit {
            jdk_home: Some("/jdk".into()),
            ..unit(&[])
        },
        &[],
    );
    assert!(classpath.contains_class(&entry(
        "/jdk/lib/modules",
        "java.base/java/lang/Object.class"
    )));
    assert!(!classpath.contains_class(&entry(
        "/other-jdk/lib/modules",
        "java.base/java/lang/Object.class"
    )));
    assert!(!classpath.contains_class(&entry(
        "/jdk/jmods/java.base.jmod",
        "classes/java/lang/Object.class"
    )));
}

#[test]
fn dependencies_do_not_export_their_classpaths_or_jdks() {
    let classpath = classpath(
        Unit {
            depends_on: vec!["lib".into()],
            ..unit(&["/project/app"])
        },
        &[(
            "lib",
            Unit {
                classpath: vec!["/project/lib.jar".into()],
                jdk_home: Some("/jdk".into()),
                ..unit(&["/project/lib"])
            },
        )],
    );
    assert!(!classpath.contains_class(&entry("/project/lib.jar", "Feature.class")));
    assert!(!classpath.contains_class(&entry(
        "/jdk/lib/modules",
        "java.base/java/lang/Object.class"
    )));
}
