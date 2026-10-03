use beans_core::model::{classpath::Classpath, source::Source};

use crate::model::Unit;

use super::{classpath, unit};

#[test]
fn classpath_entries_admit_standalone_classes_and_classes_in_output_directories() {
    let classpath = classpath(
        Unit {
            classpath: vec!["/project/out".into(), "/project/Feature.class".into()],
            ..unit(&[])
        },
        &[],
    );

    assert!(classpath.contains(&Source::class_file("/project/Feature.class")));
    assert!(classpath.contains(&Source::class_file("/project/out/p/Feature.class")));
    assert!(classpath.contains(&Source::class_file("/project/out/p/../Feature.class")));
    assert!(!classpath.contains(&Source::class_file("/project/Other.class")));
    assert!(!classpath.contains(&Source::class_file("/project/out-other/Feature.class")));
    assert!(!classpath.contains(&Source::class_file("out/Feature.class")));
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

    for source in [
        Source::jar_entry("/project/lib.jar", "p/Feature.class"),
        Source::jmod_entry("/project/lib.jmod", "classes/p/Feature.class"),
        Source::jimage_entry("/project/modules", "java.base/p/Feature.class"),
    ] {
        assert!(classpath.contains(&source), "{source:?}");
    }
    for source in [
        Source::jar_entry("/project/other.jar", "p/Feature.class"),
        Source::jmod_entry("/project/other.jmod", "classes/p/Feature.class"),
        Source::jimage_entry("/project/other-modules", "java.base/p/Feature.class"),
        Source::jar_entry("lib.jar", "p/Feature.class"),
    ] {
        assert!(!classpath.contains(&source), "{source:?}");
    }
}

#[test]
fn a_classpath_directory_does_not_implicitly_include_archives_inside_it() {
    let classpath = classpath(
        Unit {
            classpath: vec!["/project/lib".into()],
            ..unit(&[])
        },
        &[],
    );

    assert!(!classpath.contains(&Source::jar_entry(
        "/project/lib/dependency.jar",
        "p/Feature.class"
    )));
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

    assert!(classpath.contains(&Source::jimage_entry(
        "/jdk/lib/modules",
        "java.base/java/lang/Object.class"
    )));
    assert!(!classpath.contains(&Source::jimage_entry(
        "/other-jdk/lib/modules",
        "java.base/java/lang/Object.class"
    )));
    assert!(!classpath.contains(&Source::jmod_entry(
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

    assert!(!classpath.contains(&Source::jar_entry("/project/lib.jar", "Feature.class")));
    assert!(!classpath.contains(&Source::jimage_entry(
        "/jdk/lib/modules",
        "java.base/java/lang/Object.class"
    )));
}
