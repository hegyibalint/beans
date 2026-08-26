//! JLS §6.5.7 and §15.12: what a name means when it is invoked.
//!
//! Which class to search is §15.12.1 and is answered here. Which overload
//! applies is §15.12.2, needs the types of the arguments, and is not built —
//! `find_member` hands back every method of the name and lets the caller decide.

use crate::model;

use super::Wanted;

/// One method callable without a receiver.
pub(crate) struct InScopeMethod {
    pub(crate) name: String,
    pub(crate) declaration: model::DeclarationId,
}

/// The methods an unqualified call can reach: those of the type whose body
/// encloses the point, which is §15.12.1's "the class to search" for the
/// implicit `this` receiver.
///
/// Overloads all come back. §15.12.2 picks between them from the types of the
/// arguments, and an unwritten call has none — so a caller that wants one
/// method has to choose, and a caller that is offering a list shows them all.
///
/// No inheritance: §8.2 puts a superclass's methods in scope too and we do not
/// walk the hierarchy yet.
pub(crate) fn methods_in_scope<'a>(
    file: &'a model::File,
    scope: model::LexicalScopeId,
    wanted: Wanted<'a>,
) -> impl Iterator<Item = InScopeMethod> + 'a {
    file.enclosing_type_declaration(scope)
        .and_then(|type_declaration| {
            let model::Declaration::Type(declaration) = &file.declarations[type_declaration.0]
            else {
                return None;
            };
            Some(declaration.body_scope)
        })
        .into_iter()
        .flat_map(move |body_scope| {
            file.lexical_scopes[body_scope.0]
                .declarations
                .iter()
                .copied()
                .filter_map(move |member_id| {
                    let member = &file.declarations[member_id.0];
                    if member.namespace() != model::Namespace::Method {
                        return None;
                    }
                    let name = member.name()?;
                    wanted.matches(&name.text).then(|| InScopeMethod {
                        name: name.text.clone(),
                        declaration: member_id,
                    })
                })
        })
}

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
