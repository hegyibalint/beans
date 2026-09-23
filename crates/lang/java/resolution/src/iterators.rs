use beans_lang_java_model::{nodes::NodeIndex, references::TypeRef};
use beans_lang_java_semantics::query::DeclarationHandle;

use super::{Context, JavaTypeCandidate, TypeCandidate};
use crate::TypeParameterHandle;

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
    (handle.revision() == ctx.revision && handle.source() == ctx.source).then_some(handle)
}

pub(super) fn local_type_candidate(ctx: &Context<'_>, index: NodeIndex) -> Option<TypeCandidate> {
    ctx.file.node(index)?.kind().as_type()?;
    Some(TypeCandidate::Java(JavaTypeCandidate::Declaration(
        DeclarationHandle::new(ctx.revision, ctx.source.clone(), index),
    )))
}

#[cfg(test)]
mod tests {
    use beans_core_engine::Revision;
    use beans_core_model::{names::Name, source::Source};
    use beans_lang_java_model::{File, nodes::NodeIndex};

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

    #[test]
    fn direct_supertype_refs_preserve_superclass_then_interface_order() {
        let file = beans_lang_java_semantics::lower_into(
            "class Outer { class Base {} interface First {} interface Second {} class Child extends Base implements First, Second {} }",
        );
        let source = Source::SourceFile {
            path: "Test.java".into(),
        };
        let revision = Revision::new(1);
        let child = type_index(&file, &["Outer", "Child"]);
        let declaration = file.node(child).unwrap().kind().as_type().unwrap();
        let ctx = Context::new(
            revision,
            &source,
            &file,
            child,
            declaration.declared_superclass.as_ref().unwrap().value(),
        );
        let owner = TypeCandidate::Java(JavaTypeCandidate::Declaration(DeclarationHandle::new(
            revision,
            source.clone(),
            child,
        )));

        let supertypes: Vec<_> = iter_direct_supertype_refs(&ctx, &owner).collect();

        assert_eq!(supertypes.len(), 3);
        assert!(std::ptr::eq(
            supertypes[0],
            declaration.declared_superclass.as_ref().unwrap().value()
        ));
        assert!(std::ptr::eq(
            supertypes[1],
            declaration.declared_superinterfaces[0].value()
        ));
        assert!(std::ptr::eq(
            supertypes[2],
            declaration.declared_superinterfaces[1].value()
        ));
    }
}
