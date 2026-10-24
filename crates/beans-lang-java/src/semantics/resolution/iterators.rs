use crate::model::{nodes::NodeIndex, references::TypeRef};
use crate::semantics::DeclarationHandle;

use super::TypeParameterHandle;
use super::{Context, JavaTypeCandidate, TypeCandidate};

pub(super) fn iter_enclosing_types<'ctx>(
    ctx: &'ctx Context<'_>,
) -> impl Iterator<Item = TypeCandidate> + 'ctx {
    ctx.file
        .iter_ancestors(ctx.node_index)
        .filter_map(move |entry| local_type_candidate(ctx, entry.index))
}

pub(super) fn iter_direct_supertype_refs<'ctx>(
    ctx: &'ctx Context<'_>,
    owner: &TypeCandidate,
) -> impl Iterator<Item = &'ctx TypeRef> + 'ctx {
    local_declaration_handle(ctx, owner)
        .and_then(|owner| ctx.file.node(owner.node_index()))
        .and_then(|node| node.kind().as_type())
        .into_iter()
        .flat_map(|declaration| {
            declaration
                .declared_superclass
                .iter()
                .chain(&declaration.declared_superinterfaces)
                .map(|reference| reference.value())
        })
}

pub(super) fn iter_type_parameters(
    ctx: &Context<'_>,
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

pub(super) fn iter_types_below<'ctx>(
    ctx: &'ctx Context<'_>,
    parent: NodeIndex,
) -> impl Iterator<Item = TypeCandidate> + 'ctx {
    ctx.file
        .iter_children(parent)
        .filter_map(move |entry| local_type_candidate(ctx, entry.index))
}

pub(super) fn iter_declared_member_types<'ctx>(
    ctx: &'ctx Context<'_>,
    owner: &TypeCandidate,
) -> impl Iterator<Item = TypeCandidate> + 'ctx {
    local_declaration_handle(ctx, owner)
        .map(DeclarationHandle::node_index)
        .into_iter()
        .flat_map(move |owner| iter_types_below(ctx, owner))
}

pub(super) fn local_declaration_handle<'a>(
    ctx: &Context<'_>,
    candidate: &'a TypeCandidate,
) -> Option<&'a DeclarationHandle> {
    let TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)) = candidate else {
        return None;
    };
    (handle.source() == ctx.source).then_some(handle)
}

pub(super) fn local_type_candidate(ctx: &Context<'_>, index: NodeIndex) -> Option<TypeCandidate> {
    ctx.file.node(index)?.kind().as_type()?;
    Some(TypeCandidate::Java(JavaTypeCandidate::Declaration(
        DeclarationHandle::new(ctx.source.clone(), index),
    )))
}
