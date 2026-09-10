use super::super::{ResolutionInstance, ResolutionResult, Resolver};
use crate::query::JavaQuery;
use beans_core_engine::{Revision, storage::RevisionedStorage};
use beans_lang_java_model::{
    File, ScopeEntry,
    declarations::{
        Declaration,
        types::{Kind, TypeDeclaration},
    },
    references::TypeRef,
    scopes::ScopeKind,
};

fn run_stages(
    stages: &[fn(&ResolutionInstance<'_>, ScopeEntry<'_>, &str) -> ResolutionResult],
) -> ResolutionResult {
    let mut file = File::new();
    let owner = file.add_declaration(
        File::ROOT_SCOPE_ID,
        Declaration::Type(TypeDeclaration::new(Kind::Class)),
    );
    let occurrence_scope = file.new_child_scope(File::ROOT_SCOPE_ID, ScopeKind::TypeBody { owner });
    let resolver = Resolver {};
    let classpath = beans_core_model::classpath::Classpath::default();
    let files = RevisionedStorage::default();
    let query = JavaQuery::new(&files, Revision::default(), &classpath);
    let type_ref = TypeRef::Void;
    let instance =
        ResolutionInstance::new(&resolver, &file, occurrence_scope, &type_ref, &query);
    let entry = file.iter_scopes_from(File::ROOT_SCOPE_ID).next().unwrap();

    instance.find_first(entry, "A", stages)
}

fn not_found(_: &ResolutionInstance<'_>, _: ScopeEntry<'_>, _: &str) -> ResolutionResult {
    ResolutionResult::NotFound
}

fn resolved(
    instance: &ResolutionInstance<'_>,
    entry: ScopeEntry<'_>,
    name: &str,
) -> ResolutionResult {
    assert_eq!(entry.scope_index, File::ROOT_SCOPE_ID);
    assert_ne!(entry.scope_index, instance.scope_index);
    assert!(std::ptr::eq(
        entry.scope,
        instance.file.scope(File::ROOT_SCOPE_ID).unwrap()
    ));
    assert_eq!(name, "A");
    ResolutionResult::Resolved
}

fn ambiguous(_: &ResolutionInstance<'_>, _: ScopeEntry<'_>, _: &str) -> ResolutionResult {
    ResolutionResult::Ambigous
}

fn must_not_run(_: &ResolutionInstance<'_>, _: ScopeEntry<'_>, _: &str) -> ResolutionResult {
    panic!("lookup should have stopped before this stage")
}

#[test]
fn empty_stages_are_not_found() {
    assert!(matches!(run_stages(&[]), ResolutionResult::NotFound));
}

#[test]
fn exhausted_stages_are_not_found() {
    assert!(matches!(
        run_stages(&[not_found, not_found]),
        ResolutionResult::NotFound
    ));
}

#[test]
fn not_found_continues_with_the_same_scope_and_name() {
    assert!(matches!(
        run_stages(&[not_found, resolved]),
        ResolutionResult::Resolved
    ));
}

#[test]
fn resolved_stops_before_later_stages() {
    assert!(matches!(
        run_stages(&[resolved, must_not_run]),
        ResolutionResult::Resolved
    ));
}

#[test]
fn ambiguity_stops_before_later_stages() {
    assert!(matches!(
        run_stages(&[not_found, ambiguous, must_not_run]),
        ResolutionResult::Ambigous
    ));
}
