//! JLS §6.5.7 and §15.12: what a name means when it is invoked.
//!
//! Which class to search is §15.12.1 and is answered here. Which overload
//! applies is §15.12.2, needs the types of the arguments, and is not built —
//! `find_member` hands back every method of the name and lets the caller decide.

use crate::model;

/// The members of a type with a matching name in the given namespace.
/// No inheritance yet: only the type's own body scope is searched.
pub(crate) fn find_member(
    file: &model::File,
    type_declaration: model::DeclarationId,
    name: &model::Identifier,
    namespace: model::Namespace,
) -> Vec<model::DeclarationId> {
    let model::Declaration::Type(declaration) = &file.declarations[type_declaration.0] else {
        return Vec::new();
    };

    file.lexical_scopes[declaration.body_scope.0]
        .declarations
        .iter()
        .copied()
        .filter(|member_id| {
            let member = &file.declarations[member_id.0];
            member.namespace() == namespace && member.name().is_some_and(|n| n.text == name.text)
        })
        .collect()
}
