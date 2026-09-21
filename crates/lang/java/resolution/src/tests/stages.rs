use super::super::{ResolutionInstance, ResolutionResult, Resolver};
use crate::query::JavaQuery;
use crate::resolution::{ResolvedDeclarationHandle, ResolvedType, ResolvedTypeParameter};
use beans_core_engine::{Revision, storage::RevisionedStorage};
use beans_lang_java_model::{
    File, NodeEntry,
    nodes::{
        NodeKind,
        types::{Kind, TypeDeclaration},
    },
    references::TypeRef,
};

fn run_stages(
    stages: &[fn(&ResolutionInstance<'_>, NodeEntry<'_>, &str) -> ResolutionResult],
) -> ResolutionResult {
    let mut file = File::new();
    let owner = file.add_node(
        File::ROOT_NODE_ID,
        NodeKind::Type(TypeDeclaration::new(Kind::Class)),
    );
    file.add_node(
        File::ROOT_NODE_ID,
        NodeKind::Type(TypeDeclaration::new(Kind::Class)),
    );
    let resolver = Resolver {};
    let classpath = beans_core_model::classpath::Classpath::default();
    let mut files = RevisionedStorage::default();
    let source = beans_core_model::source::Source::SourceFile {
        path: "src/Use.java".into(),
    };
    files.put(Revision::default(), source.clone(), file);
    let query = JavaQuery::new(&files, Revision::default(), &classpath);
    let file = query.file(&source).unwrap();
    let type_ref = TypeRef::Void;
    let instance = ResolutionInstance::new(&resolver, &source, file, owner, &type_ref, &query);
    let entry = file.iter_ancestors(File::ROOT_NODE_ID).next().unwrap();
    instance.find_first(entry, "A", stages)
}

fn not_found(_: &ResolutionInstance<'_>, _: NodeEntry<'_>, _: &str) -> ResolutionResult {
    ResolutionResult::NotFound
}

fn resolved(
    instance: &ResolutionInstance<'_>,
    entry: NodeEntry<'_>,
    name: &str,
) -> ResolutionResult {
    assert_eq!(entry.index, File::ROOT_NODE_ID);
    assert_ne!(entry.index, instance.node_index);
    assert!(std::ptr::eq(
        entry.node,
        instance.file.node(File::ROOT_NODE_ID).unwrap()
    ));
    assert_eq!(name, "A");
    ResolutionResult::Resolved(ResolvedTypeParameter::Type(ResolvedType {
        declaration: ResolvedDeclarationHandle::Java(
            instance
                .query
                .declaration_handle(instance.source, instance.node_index),
        ),
        arguments: Default::default(),
    }))
}

fn ambiguous(instance: &ResolutionInstance<'_>, _: NodeEntry<'_>, _: &str) -> ResolutionResult {
    let candidates = instance
        .file
        .iter_children(File::ROOT_NODE_ID)
        .map(|entry| ResolvedType {
            declaration: ResolvedDeclarationHandle::Java(
                instance
                    .query
                    .declaration_handle(instance.source, entry.index),
            ),
            arguments: Default::default(),
        })
        .collect();
    ResolutionResult::Ambiguous(candidates)
}

fn must_not_run(_: &ResolutionInstance<'_>, _: NodeEntry<'_>, _: &str) -> ResolutionResult {
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
        ResolutionResult::Resolved(_)
    ));
}

#[test]
fn resolved_stops_before_later_stages() {
    assert!(matches!(
        run_stages(&[resolved, must_not_run]),
        ResolutionResult::Resolved(_)
    ));
}

#[test]
fn ambiguity_stops_before_later_stages() {
    assert!(matches!(
        run_stages(&[not_found, ambiguous, must_not_run]),
        ResolutionResult::Ambiguous(_)
    ));
}
