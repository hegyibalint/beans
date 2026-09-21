use beans_lang_java_model::nodes::{
    NodeIndex,
    types::{Kind, Modifier},
};

use crate::{
    JavaTypeCandidate, ResolutionFailure, ResolverContext, TypeCandidate, TypeParameterHandle,
};

pub(super) fn validate_resolution(
    ctx: &ResolverContext<'_>,
    resolution: TypeCandidate,
) -> Result<TypeCandidate, ResolutionFailure> {
    validate_type_parameter_usage(ctx, &resolution)?;
    Ok(resolution)
}

fn validate_type_parameter_usage(
    ctx: &ResolverContext<'_>,
    resolution: &TypeCandidate,
) -> Result<(), ResolutionFailure> {
    let TypeCandidate::Java(JavaTypeCandidate::TypeParameter(parameter)) = resolution else {
        return Ok(());
    };

    if type_parameter_is_usable(ctx, parameter) {
        Ok(())
    } else {
        Err(ResolutionFailure::IllegalTypeParameterUse(
            parameter.clone(),
        ))
    }
}

fn type_parameter_is_usable(ctx: &ResolverContext<'_>, parameter: &TypeParameterHandle) -> bool {
    let enclosing_types = enclosing_type_indices(ctx);
    let Some(owner_position) = enclosing_types
        .iter()
        .position(|index| *index == parameter.owner().node_index())
    else {
        return false;
    };

    enclosing_types[..=owner_position]
        .windows(2)
        .all(|edge| is_direct_inner_class(ctx, edge[0], edge[1]))
}

fn enclosing_type_indices(ctx: &ResolverContext<'_>) -> Vec<NodeIndex> {
    let current_type = ctx
        .file
        .node(ctx.node_index)
        .and_then(|node| node.kind().as_type())
        .map(|_| ctx.node_index);

    current_type
        .into_iter()
        .chain(
            ctx.file
                .iter_ancestors(ctx.node_index)
                .filter_map(|entry| entry.node.kind().as_type().map(|_| entry.index)),
        )
        .collect()
}

fn is_direct_inner_class(ctx: &ResolverContext<'_>, child: NodeIndex, parent: NodeIndex) -> bool {
    let Some(child) = ctx.file.node(child).and_then(|node| node.kind().as_type()) else {
        return false;
    };
    let Some(parent) = ctx.file.node(parent).and_then(|node| node.kind().as_type()) else {
        return false;
    };

    child.kind == Kind::Class
        && !child.modifiers.contains(&Modifier::Static)
        && !matches!(parent.kind, Kind::Interface | Kind::AnnotationInterface)
}
