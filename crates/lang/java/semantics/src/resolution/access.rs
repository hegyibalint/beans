use super::{DeclarationHandle, LookupProblem, ReferenceLocation, ResolverContext};
use crate::query::{JavaQuery, JavaTypeEntry};
use beans_lang_java_model::{
    File,
    nodes::{
        NodeIndex, NodeKind,
        types::{AccessLevel, Kind},
    },
};

/// JLS §6.6.1: interface member types are implicitly public.
fn access_level(entry: JavaTypeEntry<'_>) -> Option<AccessLevel> {
    if let Some(access) = entry.declaration.access.first() {
        return Some(*access);
    }
    let parent = entry
        .file
        .node(entry.node_index)
        .expect("declaration entry must reference an existing node")
        .parent()
        .expect("a type declaration must have a parent node");
    match entry
        .file
        .node(parent)
        .expect("a type declaration must have an existing parent node")
        .kind()
    {
        NodeKind::Type(owner)
            if matches!(owner.kind, Kind::Interface | Kind::AnnotationInterface) =>
        {
            Some(AccessLevel::Public)
        }
        _ => None,
    }
}

/// JLS §8.5, §9.5: private members are not inherited; package access applies at each edge.
pub(super) fn is_inheritable(
    query: &JavaQuery<'_>,
    candidate: &DeclarationHandle,
    into: JavaTypeEntry<'_>,
) -> bool {
    let entry = query
        .declaration(candidate)
        .expect("declaration handle must resolve to a type declaration");
    match access_level(entry) {
        Some(AccessLevel::Public | AccessLevel::Protected) => true,
        Some(AccessLevel::Private) => false,
        None => entry.file.package_name == into.file.package_name,
    }
}

/// JLS §6.6.1. Cross-package protected access needs a separate subclass check.
pub(super) fn is_accessible(
    ctx: &ResolverContext<'_>,
    candidate: &DeclarationHandle,
) -> Result<bool, LookupProblem> {
    let entry = ctx
        .query
        .declaration(candidate)
        .expect("declaration handle must resolve to a type declaration");
    let use_file = ctx
        .query
        .file(ctx.source)
        .expect("resolver context source must exist at the query revision");
    match access_level(entry) {
        Some(AccessLevel::Public) => Ok(true),
        Some(AccessLevel::Private) => {
            let owner = top_level_type(entry.file, entry.node_index);
            let use_owner = top_level_type(use_file, ctx.node_index);
            let in_body =
                ctx.location == ReferenceLocation::Body || use_owner != Some(ctx.node_index);
            Ok(entry.source == ctx.source && owner.is_some() && owner == use_owner && in_body)
        }
        Some(AccessLevel::Protected) if entry.file.package_name != use_file.package_name => Err(
            LookupProblem::Unsupported("Cross-package protected access is not implemented"),
        ),
        _ => Ok(entry.file.package_name == use_file.package_name),
    }
}

fn top_level_type(file: &File, node: NodeIndex) -> Option<NodeIndex> {
    file.iter_ancestors(node)
        .filter(|entry| matches!(entry.node.kind(), NodeKind::Type(_)))
        .last()
        .map(|entry| entry.index)
}
