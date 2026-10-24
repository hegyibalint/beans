use beans_core::location::{Location, LocationEntry};
use beans_workspace::Workspace;

use super::workspace;

fn file(path: &str) -> Location {
    Location::new(LocationEntry::File(path.into()))
}

fn entry(path: &str, name: &str) -> Location {
    file(path).entry(LocationEntry::Container(name.into()))
}

#[test]
fn classpath_entries_admit_standalone_classes_and_classes_in_output_directories() {
    let workspace =
        workspace("[unit.app]\nsources = ['src']\nclasspath = ['out', 'Feature.class']\n");
    let classpath = workspace.classpath_for("file:///project/src/Use.java");
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
    let workspace = workspace(
        "[unit.app]\nsources = ['src']\nclasspath = ['lib.jar', 'lib.jmod', 'modules']\n",
    );
    let classpath = workspace.classpath_for("file:///project/src/Use.java");
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
    let workspace = workspace("[unit.app]\nsources = ['src']\nclasspath = ['lib']\n");
    let classpath = workspace.classpath_for("file:///project/src/Use.java");
    for path in ["/project/lib/dependency.jar", "/project/src/dependency.jar"] {
        assert!(!classpath.contains_class(&entry(path, "p/Feature.class")));
    }
}

#[test]
fn nested_entries_follow_the_explicitly_included_outer_artifact() {
    let workspace = workspace("[unit.app]\nsources = ['src']\nclasspath = ['app.zip']\n");
    let classpath = workspace.classpath_for("file:///project/src/Use.java");
    let nested = entry("/project/app.zip", "lib/a.jar")
        .entry(LocationEntry::Container("p/Feature.class".into()));
    let other = entry("/other/app.zip", "lib/a.jar")
        .entry(LocationEntry::Container("p/Feature.class".into()));
    assert!(classpath.contains_class(&nested));
    assert!(!classpath.contains_class(&other));
}

#[test]
fn a_jdk_contributes_only_its_own_runtime_image() {
    let workspace = workspace("[unit.app]\nsources = ['src']\njdk_home = '/jdk'\n");
    let classpath = workspace.classpath_for("file:///project/src/Use.java");
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
    let workspace = workspace(
        r#"
        [unit.app]
        sources = ["app"]
        depends_on = ["lib"]
        [unit.lib]
        sources = ["lib"]
        classpath = ["lib.jar"]
        jdk_home = "/jdk"
        "#,
    );
    let classpath = workspace.classpath_for("file:///project/app/Use.java");
    assert!(!classpath.contains_class(&entry("/project/lib.jar", "Feature.class")));
    assert!(!classpath.contains_class(&entry(
        "/jdk/lib/modules",
        "java.base/java/lang/Object.class"
    )));
}
