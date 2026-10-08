use std::{collections::BTreeMap, path::PathBuf};

use beans_core::resource::{ResourceEntry, ResourceId, ResourceRoot};
use beans_workspace::Workspace;

use crate::{
    engine::TomlWorkspace,
    model::{Project, Unit},
};

fn entry(path: &str, name: &str) -> ResourceId {
    ResourceId::new(ResourceRoot::File { path: path.into() }).entry(ResourceEntry(name.into()))
}

fn unit(sources: &[&str], artifact: &str) -> Unit {
    Unit {
        sources: sources.iter().map(PathBuf::from).collect(),
        depends_on: Vec::new(),
        classpath: vec![artifact.into()],
        jdk_home: None,
    }
}

fn workspace(units: &[(&str, Unit)]) -> TomlWorkspace {
    TomlWorkspace::new(Project {
        root: "/project".into(),
        units: units
            .iter()
            .map(|(id, unit)| ((*id).into(), unit.clone()))
            .collect::<BTreeMap<_, _>>(),
    })
}

#[test]
fn each_document_selects_its_own_units_policy() {
    let workspace = workspace(&[
        ("app", unit(&["/project/app"], "/project/app.jar")),
        ("lib", unit(&["/project/lib"], "/project/lib.jar")),
    ]);

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
    let workspace = workspace(&[
        ("app", unit(&["/project/shared"], "/project/app.jar")),
        ("lib", unit(&["/project/shared"], "/project/lib.jar")),
    ]);
    let classpath = workspace.classpath_for("file:///project/shared/Future.java");

    assert!(classpath.contains_class(&entry("/project/app.jar", "Example.class")));
    assert!(classpath.contains_class(&entry("/project/lib.jar", "Example.class")));
    assert!(!classpath.contains_class(&entry("/project/other.jar", "Example.class")));
}

#[test]
fn nested_roots_add_owners_only_within_the_nested_tree() {
    let workspace = workspace(&[
        ("broad", unit(&["/project/src"], "/project/broad.jar")),
        (
            "nested",
            unit(&["/project/src/nested"], "/project/nested.jar"),
        ),
    ]);
    let nested = workspace.classpath_for("file:///project/src/nested/Future.java");
    let outer = workspace.classpath_for("file:///project/src/Future.java");

    assert!(nested.contains_class(&entry("/project/broad.jar", "Example.class")));
    assert!(nested.contains_class(&entry("/project/nested.jar", "Example.class")));
    assert!(outer.contains_class(&entry("/project/broad.jar", "Example.class")));
    assert!(!outer.contains_class(&entry("/project/nested.jar", "Example.class")));
}

#[test]
fn unclaimed_and_non_file_documents_remain_unrestricted() {
    let workspace = workspace(&[("app", unit(&["/project/src"], "/project/app.jar"))]);

    for uri in [
        "file:///project/scratch/Future.java",
        "file:///project/src-other/Future.java",
        "untitled:Future.java",
        "beans-jvm:///library/Example.class",
        "beans-jvm:///project/src/Future.java",
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
fn file_uris_are_decoded_before_ownership_is_selected() {
    let workspace = workspace(&[("app", unit(&["/project/my sources"], "/project/app.jar"))]);
    let classpath = workspace.classpath_for("file:///project/my%20sources/Future.java");

    assert!(classpath.contains_class(&entry("/project/app.jar", "Example.class")));
    assert!(!classpath.contains_class(&entry("/unrelated.jar", "Example.class")));
}
