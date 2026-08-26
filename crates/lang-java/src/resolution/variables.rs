//! JLS §6.5.6: what a name means when it is used as an expression.
//!
//! A variable is never reached through an import, a package or the lake, only
//! through the scope chain, so this has one stage where types have four. What it
//! shares is §6.4.1: the nearest declaration wins outright.
//!
//! And what it does not share is classification. A declaration reached through
//! your own chain is inside the compilation unit you are standing in, so §6.6.1
//! has nothing to ask and there is no valid/invalid half to carry.

use beans_core::model::Offset;

use crate::model;

use super::Wanted;

/// One variable in scope, and how far out it was found.
pub(crate) struct InScopeVariable {
    pub(crate) name: String,
    /// §6.4.1 shadows along the chain, so distance from the point is what
    /// ranks two declarations of one name.
    pub(crate) depth: usize,
    pub(crate) declaration: model::DeclarationId,
}

/// Every variable in scope at `scope`, nearest first.
///
/// `at` is where the question is being asked from, which §6.3 needs: the scope
/// of a local variable "starts at its own declarator", so a name written above
/// one does not see it. A caret asks with its own offset; a reference asks with
/// the offset of the reference.
pub(crate) fn variables_in_scope<'a>(
    file: &'a model::File,
    scope: model::LexicalScopeId,
    at: Offset,
    wanted: Wanted<'a>,
) -> impl Iterator<Item = InScopeVariable> + 'a {
    file.iter_scope_chain(scope)
        .enumerate()
        .flat_map(move |(depth, (_, scope))| {
            scope
                .declarations
                .iter()
                .copied()
                .filter_map(move |declaration_id| {
                    let declaration = file.declarations.get(declaration_id.0)?;
                    match declaration {
                        model::Declaration::Local(local) => {
                            // §6.3: the scope of a local starts at its declarator.
                            let name = local.name.as_ref()?;
                            (name.span.start <= at).then_some(name)
                        }
                        model::Declaration::Parameter(_) | model::Declaration::Field(_) => {
                            declaration.name()
                        }
                        _ => None,
                    }
                    .filter(|name| wanted.matches(&name.text))
                    .map(|name| InScopeVariable {
                        name: name.text.clone(),
                        depth,
                        declaration: declaration_id,
                    })
                })
        })
}

/// A bare name in expression position: locals, parameters, then fields,
/// nearest scope first. Always in-file.
///
/// §6.4.1 as a fold, the same shape `first_stage_that_answers` has for types:
/// the nearest scope that spells the name answers, and no farther one is asked.
pub(crate) fn resolve_variable_name(
    file: &model::File,
    name: &model::Identifier,
    scope: model::LexicalScopeId,
) -> Vec<model::DeclarationId> {
    let mut found: Vec<model::DeclarationId> = Vec::new();
    let mut depth = None;

    for variable in variables_in_scope(file, scope, name.span.start, Wanted::Exactly(&name.text)) {
        if depth.is_some_and(|found| found != variable.depth) {
            break;
        }
        depth = Some(variable.depth);
        found.push(variable.declaration);
    }

    found
}
