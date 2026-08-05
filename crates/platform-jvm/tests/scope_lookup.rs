//! Where a source's scope comes from. `class_lookup` hands a scope in by hand;
//! here the store answers, which is what a query will actually do.
//!
//! The project is `app -> lib -> core`, plus a `common` tree that both `app`
//! and `lib` declare as their own. Every class sits in package `p`, so a
//! package boundary can never be what stops a lookup.

use std::path::PathBuf;

use beans_core::storage::Revision;
use beans_platform_jvm::PlatformJvm;
use beans_platform_jvm::model::{JvmAccessLevel, JvmClass, JvmKind, JvmQualifiedName, JvmSource};
use beans_platform_jvm::query::{JvmContainer, JvmScope, JvmScopeMembership};

const APP: &str = "app/src";
const LIB: &str = "lib/src";
const CORE: &str = "core/src";
const COMMON: &str = "common/src";

fn source_file(tree: &str, relative: &str) -> JvmSource {
    JvmSource::SourceFile {
        path: PathBuf::from(tree).join(relative),
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

fn tree(directory: &str) -> JvmContainer {
    JvmContainer::Source(PathBuf::from(directory))
}

fn app() -> JvmSource {
    source_file(APP, "p/App.java")
}

fn lib() -> JvmSource {
    source_file(LIB, "p/Lib.java")
}

fn core() -> JvmSource {
    source_file(CORE, "p/Core.java")
}

fn shared() -> JvmSource {
    source_file(COMMON, "p/Shared.java")
}

/// The four files in the lake, and nothing scoped yet.
fn lake() -> (PlatformJvm, Revision) {
    let mut jvm = PlatformJvm::new();
    let mut revision = Revision::default();

    jvm.register(revision.bump(), app(), vec![class("p.App")]);
    jvm.register(revision.bump(), lib(), vec![class("p.Lib")]);
    jvm.register(revision.bump(), core(), vec![class("p.Core")]);
    jvm.register(revision.bump(), shared(), vec![class("p.Shared")]);

    (jvm, revision)
}

/// The graph flattened, which is the only place dependency edges exist. `app`
/// lists `lib` and `core` because it reaches them through `lib`; that walk
/// happens here and never again.
fn scoped() -> (PlatformJvm, Revision) {
    let (mut jvm, mut revision) = lake();

    let core_scope = JvmScope::of(vec![tree(CORE)]);
    let lib_scope = JvmScope::of(vec![tree(LIB), tree(COMMON), tree(CORE)]);
    let app_scope = JvmScope::of(vec![tree(APP), tree(COMMON), tree(LIB), tree(CORE)]);

    let revision = revision.bump();
    jvm.register_scopes(revision, app(), vec![app_scope.clone()]);
    jvm.register_scopes(revision, lib(), vec![lib_scope.clone()]);
    jvm.register_scopes(revision, core(), vec![core_scope]);
    // Checked into both, so it carries both.
    jvm.register_scopes(revision, shared(), vec![app_scope, lib_scope]);

    (jvm, revision)
}

fn sees(jvm: &PlatformJvm, revision: Revision, asker: &JvmSource, fqn: &str) -> bool {
    let query = jvm.query_from(asker, revision);
    query
        .classes_named(&JvmQualifiedName::new(fqn))
        .into_iter()
        .any(|(source, _)| query.scope_membership(source) == JvmScopeMembership::InScope)
}

#[test]
fn a_source_nobody_scoped_sees_everything() {
    let (jvm, revision) = lake();

    assert!(sees(&jvm, revision, &core(), "p.App"));
    assert!(sees(&jvm, revision, &core(), "p.Lib"));
}

/// The point of the whole exercise. `app` reaches `core`, and `core` cannot
/// reach back, because a dependency edge points one way.
#[test]
fn visibility_runs_one_way() {
    let (jvm, revision) = scoped();

    assert!(sees(&jvm, revision, &app(), "p.Core"));
    let query = jvm.query_from(&core(), revision);
    let app = query.classes_named(&JvmQualifiedName::new("p.App"));
    assert_eq!(app.len(), 1);
    assert_eq!(
        query.scope_membership(app[0].0),
        JvmScopeMembership::OutsideScope
    );
}

#[test]
fn a_unit_does_not_reach_a_sibling_it_never_declared() {
    let (jvm, revision) = scoped();

    assert!(sees(&jvm, revision, &lib(), "p.Core"));
    assert!(!sees(&jvm, revision, &lib(), "p.App"));
}

/// Shared code carries a scope per unit that claims it, and the union is what
/// it can reach. `p.App` is only in one of the two, and one is enough.
#[test]
fn a_file_two_units_claim_sees_the_union() {
    let (jvm, revision) = scoped();

    assert!(sees(&jvm, revision, &shared(), "p.App"));
    assert!(sees(&jvm, revision, &shared(), "p.Core"));
}

#[test]
fn a_shared_tree_is_reachable_from_every_unit_that_claims_it() {
    let (jvm, revision) = scoped();

    assert!(sees(&jvm, revision, &app(), "p.Shared"));
    assert!(sees(&jvm, revision, &lib(), "p.Shared"));
    assert!(!sees(&jvm, revision, &core(), "p.Shared"));
}

/// Scopes are revisioned like everything else, so a read from before the
/// import lands still sees the unscoped world rather than a half-built one.
#[test]
fn a_read_older_than_the_scopes_is_unscoped() {
    let (jvm, revision) = scoped();
    let before_scopes = Revision::default().bump().bump().bump().bump();

    assert!(!sees(&jvm, revision, &core(), "p.App"));
    assert!(sees(&jvm, before_scopes, &core(), "p.App"));
}

/// A source's scope list is its whole answer, so re-registering replaces it.
/// That is what a workspace change has to rely on.
#[test]
fn re_registering_replaces_the_whole_scope() {
    let (mut jvm, mut revision) = scoped();

    let revision = revision.bump();
    jvm.register_scopes(revision, core(), vec![JvmScope::of(vec![tree(APP)])]);

    assert!(sees(&jvm, revision, &core(), "p.App"));
    assert!(!sees(&jvm, revision, &core(), "p.Core"));
}

/// Registering no scopes is not the same as registering nothing: an entry with
/// an empty list says the source sees nothing at all, where no entry says
/// nobody has placed it and it sees everything. Whoever flattens a workspace
/// has to leave an unplaced file unregistered, and this is the difference that
/// makes it matter.
#[test]
fn an_empty_scope_list_sees_nothing() {
    let (mut jvm, mut revision) = lake();

    let revision = revision.bump();
    jvm.register_scopes(revision, app(), Vec::new());

    assert!(!sees(&jvm, revision, &app(), "p.App"));
    assert!(
        sees(&jvm, revision, &lib(), "p.App"),
        "lib is still unscoped"
    );
}

/// Nothing places a jar entry, so one asking a question is unscoped and the
/// whole lake answers.
#[test]
fn a_source_a_workspace_never_places_stays_unscoped() {
    let (jvm, revision) = scoped();
    let entry = JvmSource::JarEntry {
        jar_path: PathBuf::from("lib.jar"),
        entry_path: "p/Other.class".to_string(),
    };

    assert!(sees(&jvm, revision, &entry, "p.App"));
}
