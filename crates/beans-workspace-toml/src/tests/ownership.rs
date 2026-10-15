use beans_core::resource::ResourceId;
use beans_workspace::Workspace;

use crate::{
    TomlWorkspace,
    model::{Module, Project},
};

#[test]
fn each_document_selects_its_own_modules_policy() {
    let workspace = workspace(&[
        ("app", module(&["/project/app"], &[])),
        ("lib", module(&["/project/lib"], &[])),
    ]);

    for (document, visible, hidden) in [
        (
            "file:///project/app/Future.java",
            "/project/app/Target.java",
            "/project/lib/Target.java",
        ),
        (
            "file:///project/lib/Future.java",
            "/project/lib/Target.java",
            "/project/app/Target.java",
        ),
    ] {
        let classpath = workspace.classpath_for(document);
        assert!(classpath.contains_source(&ResourceId::file(visible)));
        assert!(!classpath.contains_source(&ResourceId::file(hidden)));
    }
}

#[test]
fn shared_roots_combine_the_policies_of_all_owners() {
    let workspace = workspace(&[
        ("app", module(&["/project/shared"], &["app-dep"])),
        ("lib", module(&["/project/shared"], &["lib-dep"])),
        ("app-dep", module(&["/project/app-dep"], &[])),
        ("lib-dep", module(&["/project/lib-dep"], &[])),
    ]);
    let classpath = workspace.classpath_for("file:///project/shared/Future.java");

    assert!(classpath.contains_source(&ResourceId::file("/project/app-dep/Target.java")));
    assert!(classpath.contains_source(&ResourceId::file("/project/lib-dep/Target.java")));
    assert!(!classpath.contains_source(&ResourceId::file("/project/other/Target.java")));
}

#[test]
fn nested_roots_add_owners_only_within_the_nested_tree() {
    let workspace = workspace(&[
        ("broad", module(&["/project/src"], &["broad-dep"])),
        ("nested", module(&["/project/src/nested"], &["nested-dep"])),
        ("broad-dep", module(&["/project/broad-dep"], &[])),
        ("nested-dep", module(&["/project/nested-dep"], &[])),
    ]);
    let nested = workspace.classpath_for("file:///project/src/nested/Future.java");
    let outer = workspace.classpath_for("file:///project/src/Future.java");

    assert!(nested.contains_source(&ResourceId::file("/project/broad-dep/Target.java")));
    assert!(nested.contains_source(&ResourceId::file("/project/nested-dep/Target.java")));
    assert!(outer.contains_source(&ResourceId::file("/project/broad-dep/Target.java")));
    assert!(!outer.contains_source(&ResourceId::file("/project/nested-dep/Target.java")));
}

#[test]
fn unclaimed_and_non_file_documents_remain_unrestricted() {
    let workspace = workspace(&[("app", module(&["/project/src"], &[]))]);

    for uri in [
        "file:///project/scratch/Future.java",
        "file:///project/src-other/Future.java",
        "untitled:Future.java",
        "beans-jvm:///project/src/Future.java",
        "not a URI",
    ] {
        assert!(
            workspace
                .classpath_for(uri)
                .contains_source(&ResourceId::file("/unrelated/Target.java")),
            "{uri}"
        );
    }
}

#[test]
fn file_uris_are_decoded_before_ownership_is_selected() {
    let workspace = workspace(&[("app", module(&["/project/my sources"], &[]))]);
    let classpath = workspace.classpath_for("file:///project/my%20sources/Future.java");

    assert!(classpath.contains_source(&ResourceId::file("/project/my sources/Target.java")));
    assert!(!classpath.contains_source(&ResourceId::file("/unrelated/Target.java")));
}

fn module(sources: &[&str], depends_on: &[&str]) -> Module {
    Module {
        sources: sources.iter().map(Into::into).collect(),
        depends_on: depends_on.iter().map(|name| (*name).into()).collect(),
    }
}

fn workspace(modules: &[(&str, Module)]) -> TomlWorkspace {
    TomlWorkspace::new(Project {
        root: "/project".into(),
        jdk_home: None,
        modules: modules
            .iter()
            .map(|(id, module)| ((*id).into(), module.clone()))
            .collect(),
    })
}
