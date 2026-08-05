//! Two jars both declare `p.B`, and each belongs to a different scope.
//! Discovery returns both; composing it with the asking query's membership
//! selects the one its scope holds. The rest is the lake underneath: what a
//! registration replaces, and what a read at an older revision sees.

use std::path::PathBuf;

use beans_core::storage::Revision;
use beans_platform_jvm::PlatformJvm;
use beans_platform_jvm::model::{JvmAccessLevel, JvmClass, JvmKind, JvmQualifiedName, JvmSource};
use beans_platform_jvm::query::{
    JvmContainer, JvmQuery, JvmScope, JvmScopeMembership, JvmScopeQuery,
};

const JAR_ONE: &str = "lib-one-1.0.jar";
const JAR_TWO: &str = "lib-two-1.0.jar";
const SOURCES: &str = "app/src/main/java";

fn source_file(relative: &str) -> JvmSource {
    JvmSource::SourceFile {
        path: PathBuf::from(SOURCES).join(relative),
    }
}

fn jar_entry(jar: &str, entry: &str) -> JvmSource {
    JvmSource::JarEntry {
        jar_path: PathBuf::from(jar),
        entry_path: entry.to_string(),
    }
}

fn class(fqn: &str) -> JvmClass {
    JvmClass {
        fqn: JvmQualifiedName::new(fqn),
        kind: JvmKind::Class,
        access: Some(JvmAccessLevel::Public),
        enclosing: None,
        superclass: None,
        interfaces: Vec::new(),
        fields: Vec::new(),
        methods: Vec::new(),
    }
}

/// `A` in the workspace, and `p.B` declared once in each jar.
fn lake() -> (PlatformJvm, Revision) {
    let mut jvm = PlatformJvm::new();
    let mut revision = Revision::default();

    jvm.register(revision.bump(), source_file("p/A.java"), vec![class("p.A")]);
    jvm.register(
        revision.bump(),
        jar_entry(JAR_ONE, "p/B.class"),
        vec![class("p.B")],
    );
    jvm.register(
        revision.bump(),
        jar_entry(JAR_TWO, "p/B.class"),
        vec![class("p.B")],
    );

    (jvm, revision)
}

/// No scope information at all: every lookup made here is asking about
/// registration rather than about visibility.
fn whole_lake(jvm: &PlatformJvm, revision: Revision) -> JvmQuery<'_> {
    JvmQuery::new(jvm, JvmScopeQuery::unscoped(), revision)
}

fn holding(jvm: &PlatformJvm, revision: Revision, containers: Vec<JvmContainer>) -> JvmQuery<'_> {
    JvmQuery::new(
        jvm,
        JvmScopeQuery::of(vec![JvmScope::of(containers)]),
        revision,
    )
}

fn artifact(jar: &str) -> JvmContainer {
    JvmContainer::Artifact(PathBuf::from(jar))
}

fn sources(directory: &str) -> JvmContainer {
    JvmContainer::Source(PathBuf::from(directory))
}

fn in_scope<'a>(
    query: &JvmQuery<'a>,
    fqn: &JvmQualifiedName,
) -> Vec<(&'a JvmSource, &'a JvmClass)> {
    query
        .classes_named(fqn)
        .into_iter()
        .filter(|(source, _)| query.scope_membership(source) == JvmScopeMembership::InScope)
        .collect()
}

#[test]
fn a_name_no_container_declares_resolves_to_nothing() {
    let (jvm, revision) = lake();

    let found = whole_lake(&jvm, revision).classes_named(&JvmQualifiedName::new("p.Missing"));

    assert!(found.is_empty());
}

#[test]
fn discovery_retains_candidates_outside_the_scope() {
    let (jvm, revision) = lake();
    let query = holding(&jvm, revision, vec![sources(SOURCES)]);

    let found = query.classes_named(&JvmQualifiedName::new("p.B"));

    assert_eq!(found.len(), 2);
    assert!(
        found
            .iter()
            .all(|(source, _)| query.scope_membership(source) == JvmScopeMembership::OutsideScope)
    );
    assert!(
        found
            .iter()
            .any(|(source, _)| **source == jar_entry(JAR_ONE, "p/B.class"))
    );
    assert!(
        found
            .iter()
            .any(|(source, _)| **source == jar_entry(JAR_TWO, "p/B.class"))
    );
}

/// Registering a source again replaces its whole contribution, so a container
/// that stops declaring `p.B` leaves the other one alone with the name.
#[test]
fn a_container_that_no_longer_declares_a_name_drops_out() {
    let (mut jvm, mut revision) = lake();

    jvm.register(
        revision.bump(),
        jar_entry(JAR_ONE, "p/B.class"),
        vec![class("p.Renamed")],
    );

    let found = whole_lake(&jvm, revision).classes_named(&JvmQualifiedName::new("p.B"));

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].0, &jar_entry(JAR_TWO, "p/B.class"));
}

/// A read at an older revision predates the second jar entirely.
#[test]
fn an_older_revision_sees_the_lake_before_the_second_jar_landed() {
    let (jvm, _) = lake();

    // A, then jar one, then jar two: two bumps in.
    let before_jar_two = Revision::default().bump().bump();
    let found = whole_lake(&jvm, before_jar_two).classes_named(&JvmQualifiedName::new("p.B"));

    assert_eq!(found.len(), 1);
    assert_eq!(found[0].0, &jar_entry(JAR_ONE, "p/B.class"));
}

#[test]
fn package_discovery_is_not_filtered_by_scope() {
    let (jvm, revision) = lake();
    let query = holding(&jvm, revision, vec![sources(SOURCES)]);

    let found = query.classes_in_package("p");
    let names: Vec<&str> = found.iter().map(|(_, class)| class.fqn.as_str()).collect();

    assert_eq!(names.len(), 3, "found {names:?}");
    assert_eq!(names.iter().filter(|name| **name == "p.B").count(), 2);
    assert_eq!(
        found
            .iter()
            .filter(|(source, _)|
                query.scope_membership(source) == JvmScopeMembership::OutsideScope)
            .count(),
        2
    );
}

/// Discovery is independent of scope; composing it with membership selects the
/// declaration available from each viewpoint.
#[test]
fn each_scope_contains_only_the_declaration_its_own_container_holds() {
    let (jvm, revision) = lake();
    let b = JvmQualifiedName::new("p.B");
    let query_from_one = holding(&jvm, revision, vec![artifact(JAR_ONE)]);
    let query_from_two = holding(&jvm, revision, vec![artifact(JAR_TWO)]);

    assert_eq!(query_from_one.classes_named(&b).len(), 2);
    assert_eq!(query_from_two.classes_named(&b).len(), 2);

    let from_one = in_scope(&query_from_one, &b);
    let from_two = in_scope(&query_from_two, &b);

    assert_eq!(from_one.len(), 1, "found {from_one:?}");
    assert_eq!(from_one[0].0, &jar_entry(JAR_ONE, "p/B.class"));

    assert_eq!(from_two.len(), 1, "found {from_two:?}");
    assert_eq!(from_two[0].0, &jar_entry(JAR_TWO, "p/B.class"));
}

/// A scope holding both is where ranking will have to take over: membership
/// alone cannot separate them.
#[test]
fn a_scope_holding_both_containers_still_sees_the_name_twice() {
    let (jvm, revision) = lake();

    let query = holding(&jvm, revision, vec![artifact(JAR_ONE), artifact(JAR_TWO)]);
    let b = JvmQualifiedName::new("p.B");
    let found = in_scope(&query, &b);

    assert_eq!(found.len(), 2, "found {found:?}");
}

/// A source tree is a container in its own right, so a scope of jars alone
/// cannot see a workspace file, and one holding the tree can.
#[test]
fn a_source_tree_is_a_container_of_its_own() {
    let (jvm, revision) = lake();
    let a = JvmQualifiedName::new("p.A");

    let jars_only = holding(&jvm, revision, vec![artifact(JAR_ONE), artifact(JAR_TWO)]);
    let found = jars_only.classes_named(&a);
    assert_eq!(found.len(), 1);
    assert_eq!(
        jars_only.scope_membership(found[0].0),
        JvmScopeMembership::OutsideScope
    );

    let with_sources = holding(&jvm, revision, vec![sources(SOURCES)]);
    assert_eq!(in_scope(&with_sources, &a).len(), 1);
}

/// A tree matches by prefix, so a sibling tree is a different container even
/// though both hold source files.
#[test]
fn a_source_tree_does_not_reach_into_a_sibling_tree() {
    let (jvm, revision) = lake();

    let elsewhere = holding(&jvm, revision, vec![sources("lib/src/main/java")]);

    let found = elsewhere.classes_named(&JvmQualifiedName::new("p.A"));
    assert_eq!(found.len(), 1);
    assert_eq!(
        elsewhere.scope_membership(found[0].0),
        JvmScopeMembership::OutsideScope
    );
}
