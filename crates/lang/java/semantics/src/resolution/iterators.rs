use beans_lang_java_model::NodeEntry;

use super::{
    JavaTypeCandidate, ResolutionFailure, ResolutionSuccess, Resolver, ResolverContext,
    TypeCandidate,
};
use crate::query::{DeclarationHandle, TypeParameterHandle};

pub(super) fn iter_enclosing_types<'ctx>(
    ctx: &'ctx ResolverContext<'_>,
) -> impl Iterator<Item = TypeCandidate> + 'ctx {
    ctx.file
        .iter_ancestors(ctx.node_index)
        .filter_map(move |entry| type_candidate(ctx, entry))
}

pub(super) fn iter_direct_supertypes<'ctx>(
    ctx: &'ctx ResolverContext<'_>,
    owner: &TypeCandidate,
) -> impl Iterator<Item = Result<ResolutionSuccess, ResolutionFailure>> + 'ctx {
    let owner = local_declaration_handle(ctx, owner);
    let owner_index = owner.map(DeclarationHandle::node_index);
    let supertypes = owner
        .and_then(|owner| ctx.file.node(owner.node_index()))
        .and_then(|node| node.kind().as_type())
        .map(|declaration| {
            declaration
                .declared_superclass
                .iter()
                .chain(&declaration.declared_superinterfaces)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    supertypes.into_iter().map(move |type_ref| {
        let supertype_ctx = ResolverContext::new(
            ctx.revision,
            ctx.source,
            ctx.file,
            owner_index.expect("supertype owner must exist"),
            type_ref,
        );
        Resolver::resolve(&supertype_ctx)
    })
}

pub(super) fn iter_type_parameters(
    ctx: &ResolverContext<'_>,
    owner: &TypeCandidate,
) -> impl Iterator<Item = TypeCandidate> {
    let Some(owner) = local_declaration_handle(ctx, owner).cloned() else {
        return Vec::new().into_iter();
    };
    let Some(declaration) = ctx
        .file
        .node(owner.node_index())
        .and_then(|node| node.kind().as_type())
    else {
        return Vec::new().into_iter();
    };

    declaration
        .type_parameters
        .iter()
        .enumerate()
        .map(|(index, _)| {
            TypeCandidate::Java(JavaTypeCandidate::TypeParameter(TypeParameterHandle::new(
                owner.clone(),
                index,
            )))
        })
        .collect::<Vec<_>>()
        .into_iter()
}

/// Iterates over the member types exposed by `owner`.
///
/// Inherited and JVM member types are not available yet.
pub(super) fn iter_member_types<'ctx>(
    ctx: &'ctx ResolverContext<'_>,
    owner: &TypeCandidate,
) -> impl Iterator<Item = TypeCandidate> + 'ctx {
    iter_declared_member_types(ctx, owner)
}

pub(super) fn iter_declared_member_types<'ctx>(
    ctx: &'ctx ResolverContext<'_>,
    owner: &TypeCandidate,
) -> impl Iterator<Item = TypeCandidate> + 'ctx {
    let owner = local_declaration_handle(ctx, owner).map(DeclarationHandle::node_index);

    owner
        .into_iter()
        .flat_map(move |owner| ctx.file.iter_children(owner))
        .filter_map(move |entry| type_candidate(ctx, entry))
}

fn local_declaration_handle<'a>(
    ctx: &ResolverContext<'_>,
    candidate: &'a TypeCandidate,
) -> Option<&'a DeclarationHandle> {
    let TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)) = candidate else {
        return None;
    };
    (handle.revision() == ctx.revision && handle.source() == ctx.source).then_some(handle)
}

fn type_candidate(ctx: &ResolverContext<'_>, entry: NodeEntry<'_>) -> Option<TypeCandidate> {
    entry.node.kind().as_type()?;
    Some(TypeCandidate::Java(JavaTypeCandidate::Declaration(
        DeclarationHandle::new(ctx.revision, ctx.source.clone(), entry.index),
    )))
}

#[cfg(test)]
mod tests {
    use beans_core_engine::Revision;
    use beans_core_model::source::Source;
    use beans_lang_java_model::{File, names::Name, nodes::NodeIndex};

    use super::*;

    fn type_index(file: &File, components: &[&str]) -> NodeIndex {
        let name = Name::new(
            components
                .iter()
                .map(|component| (*component).to_owned())
                .collect(),
        );
        let matches = file.find_type(&name);
        assert_eq!(matches.len(), 1);
        matches[0].0
    }

    fn candidate(revision: Revision, source: &Source, index: NodeIndex) -> TypeCandidate {
        TypeCandidate::Java(JavaTypeCandidate::Declaration(DeclarationHandle::new(
            revision,
            source.clone(),
            index,
        )))
    }

    #[test]
    fn direct_supertypes_are_resolved_in_declaration_order() {
        let file = crate::lower_into(
            "class Outer { class Base {} interface Contract {} class Child extends Base implements Contract {} }",
        );
        let source = Source::SourceFile {
            path: "Test.java".into(),
        };
        let revision = Revision::new(1);
        let child = type_index(&file, &["Outer", "Child"]);
        let child_declaration = file.node(child).unwrap().kind().as_type().unwrap();
        let ctx = ResolverContext::new(
            revision,
            &source,
            &file,
            child,
            child_declaration.declared_superclass.as_ref().unwrap(),
        );

        let resolved: Vec<_> = iter_direct_supertypes(&ctx, &candidate(revision, &source, child))
            .map(|result| match result.unwrap() {
                ResolutionSuccess::Single(TypeCandidate::Java(JavaTypeCandidate::Declaration(
                    handle,
                ))) => handle.node_index(),
                result => panic!("expected one Java declaration, got {result:?}"),
            })
            .collect();

        assert_eq!(
            resolved,
            [
                type_index(&file, &["Outer", "Base"]),
                type_index(&file, &["Outer", "Contract"]),
            ]
        );
    }

    #[test]
    fn direct_supertype_failures_do_not_hide_later_results() {
        let file = crate::lower_into(
            "class Outer { interface Contract {} class Child extends Missing implements Contract {} }",
        );
        let source = Source::SourceFile {
            path: "Test.java".into(),
        };
        let revision = Revision::new(1);
        let child = type_index(&file, &["Outer", "Child"]);
        let child_declaration = file.node(child).unwrap().kind().as_type().unwrap();
        let ctx = ResolverContext::new(
            revision,
            &source,
            &file,
            child,
            child_declaration.declared_superclass.as_ref().unwrap(),
        );

        let results: Vec<_> =
            iter_direct_supertypes(&ctx, &candidate(revision, &source, child)).collect();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0], Err(ResolutionFailure::NotFound));
        assert!(matches!(results[1], Ok(ResolutionSuccess::Single(_))));
    }
}
