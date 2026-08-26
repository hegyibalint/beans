//! JLS §6.5.6: what a name means when it is used as an expression.
//!
//! A variable is never reached through an import, a package or the lake, only
//! through the scope chain, so this has one stage where types have four. What it
//! shares is §6.4.1: the nearest declaration wins outright.

use crate::model;

/// A bare name in expression position: locals, parameters, then fields,
/// nearest scope first. Always in-file.
pub(crate) fn resolve_variable_name(
    file: &model::File,
    name: &model::Identifier,
    scope: model::LexicalScopeId,
) -> Vec<model::DeclarationId> {
    for (_, scope) in file.iter_scope_chain(scope) {
        let hits: Vec<model::DeclarationId> = scope
            .declarations
            .iter()
            .copied()
            .filter(|declaration_id| {
                match &file.declarations[declaration_id.0] {
                    // JLS 6.3: a local's scope starts at its declarator.
                    model::Declaration::Local(declaration) => {
                        declaration.name.as_ref().is_some_and(|local| {
                            local.text == name.text && local.span.start <= name.span.start
                        })
                    }
                    model::Declaration::Parameter(_) | model::Declaration::Field(_) => file
                        .declarations[declaration_id.0]
                        .name()
                        .is_some_and(|candidate| candidate.text == name.text),
                    _ => false,
                }
            })
            .collect();
        if !hits.is_empty() {
            return hits;
        }
    }

    Vec::new()
}
