//! Resolving an occurrence to what it denotes.
//!
//! JLS §6.5 splits this by what a name is used *as*, and so does this module.
//! §6.1's three namespaces turn out to have three different sets of rules —
//! types stage across imports, packages and the lake; a variable never leaves
//! the scope chain; a method ends in §15.12.2's applicability — so they share a
//! `Site`, a `Wanted` and the shape of the fold rather than the rules.
//!
//! What stays here is the dispatch: deciding which of the three is being asked,
//! which is §6.5.2's job and belongs to none of them.

pub(crate) mod candidates;
pub(crate) mod methods;
pub(crate) mod types;
pub(crate) mod variables;

use beans_core::language::NavigationTarget;
use beans_core::model::Offset;
use beans_platform_jvm as jvm;

use crate::{model, query::Query};

pub(crate) use candidates::{ResolutionCandidates, TypeInvalidity, first_stage_that_answers};
pub use candidates::{TypeResolution, TypeTarget};
pub(crate) use methods::find_member;
pub(crate) use types::{
    InScopeType, Wanted, resolve_type_candidates, resolve_type_name, types_in_scope,
};
pub(crate) use variables::resolve_variable_name;

use types::resolve_type_reference;

/// Resolves the occurrence at `offset` to the declarations it refers to.
/// This is the go-to-declaration entry point.
pub fn resolve_occurrence_at(
    source: &jvm::model::Source,
    file: &model::File,
    offset: Offset,
    query: &Query,
) -> Vec<NavigationTarget<jvm::model::Source>> {
    let Some((_, entity)) = file.position_index.tightest_containing(offset) else {
        return Vec::new();
    };

    let targets = match entity {
        model::EntityId::Declaration(declaration) => vec![(source.clone(), declaration)],
        model::EntityId::TypeRef(owner) => {
            let declaration = &file.declarations[owner.0];
            let Some(type_ref) = declaration.type_ref() else {
                return Vec::new();
            };
            resolve_type_reference(source, file, type_ref, declaration.declaring_scope(), query)
        }
        model::EntityId::BodyNode(body, node) => {
            let model::BodyNodeKind::Expression(_) = &file.bodies[body.0].node(node).kind else {
                return Vec::new();
            };
            resolve_expression(source, file, body, node, query)
        }
        model::EntityId::Scope(..) | model::EntityId::Import(..) => Vec::new(),
    };

    targets
        .iter()
        .filter_map(|(target_source, declaration_id)| {
            let target_file = query.model_of(target_source)?;
            let span = target_file.declarations[declaration_id.0].name_span()?;
            Some(NavigationTarget {
                source: target_source.clone(),
                span,
            })
        })
        .collect()
}

pub(crate) fn resolve_expression(
    source: &jvm::model::Source,
    file: &model::File,
    body_id: model::BodyId,
    expression_id: model::BodyNodeId,
    query: &Query,
) -> Vec<(jvm::model::Source, model::DeclarationId)> {
    let body = &file.bodies[body_id.0];
    let scope = body.node(expression_id).scope;
    let Some(expression) = body.expression(expression_id) else {
        return Vec::new();
    };
    match expression {
        model::Expression::NameRef { name } => resolve_variable_name(file, name, scope)
            .into_iter()
            .map(|declaration| (source.clone(), declaration))
            .collect(),
        model::Expression::This => file
            .enclosing_type_declaration(scope)
            .map(|declaration| vec![(source.clone(), declaration)])
            .unwrap_or_default(),
        model::Expression::FieldAccess { receiver, name } => {
            let Some((class_source, class)) =
                resolve_receiver_class(source, file, body_id, *receiver, query)
            else {
                return Vec::new();
            };
            let Some(class_file) = query.model_of(&class_source) else {
                return Vec::new();
            };
            find_member(class_file, class, name, model::Namespace::Variable)
                .into_iter()
                .map(|member| (class_source.clone(), member))
                .collect()
        }
        model::Expression::MethodCall { receiver, name, .. } => {
            let receiver_class = match receiver {
                Some(receiver) => resolve_receiver_class(source, file, body_id, *receiver, query),
                None => file
                    .enclosing_type_declaration(scope)
                    .map(|declaration| (source.clone(), declaration)),
            };
            let Some((class_source, class)) = receiver_class else {
                return Vec::new();
            };
            let Some(class_file) = query.model_of(&class_source) else {
                return Vec::new();
            };
            find_member(class_file, class, name, model::Namespace::Method)
                .into_iter()
                .map(|member| (class_source.clone(), member))
                .collect()
        }
        model::Expression::ObjectCreation { ty, .. } => {
            resolve_type_reference(source, file, ty, scope, query)
        }
        model::Expression::Assign { .. } | model::Expression::Literal => Vec::new(),
    }
}

/// The class through which member lookup for `expression` runs: the declared
/// type of the expression, or the type itself for static access (`Bar.asd`).
fn resolve_receiver_class(
    source: &jvm::model::Source,
    file: &model::File,
    body_id: model::BodyId,
    expression_id: model::BodyNodeId,
    query: &Query,
) -> Option<(jvm::model::Source, model::DeclarationId)> {
    let body = &file.bodies[body_id.0];
    let scope = body.node(expression_id).scope;
    let Some(expression) = body.expression(expression_id) else {
        return None;
    };
    match expression {
        model::Expression::This => {
            let declaration = file.enclosing_type_declaration(scope)?;
            Some((source.clone(), declaration))
        }
        model::Expression::NameRef { name } => {
            if let Some(variable) = resolve_variable_name(file, name, scope).into_iter().next() {
                let declaration = &file.declarations[variable.0];
                let type_ref = declaration.type_ref()?;
                return resolve_type_reference(
                    source,
                    file,
                    type_ref,
                    declaration.declaring_scope(),
                    query,
                )
                .into_iter()
                .next();
            }

            // Not a variable: try a type name for static access (`Bar.asd`).
            match resolve_type_name(
                &model::Name::Simple(name.clone()),
                source,
                file,
                scope,
                query,
            ) {
                TypeResolution::Resolved(TypeTarget::Parsed {
                    source,
                    declaration,
                }) => Some((source, declaration)),
                _ => None,
            }
        }
        model::Expression::FieldAccess { receiver, name } => {
            let (class_source, class) =
                resolve_receiver_class(source, file, body_id, *receiver, query)?;
            let class_file = query.model_of(&class_source)?;
            let member = find_member(class_file, class, name, model::Namespace::Variable)
                .into_iter()
                .next()?;
            let declaration = &class_file.declarations[member.0];
            resolve_type_reference(
                &class_source,
                class_file,
                declaration.type_ref()?,
                declaration.declaring_scope(),
                query,
            )
            .into_iter()
            .next()
        }
        model::Expression::MethodCall { receiver, name, .. } => {
            let receiver_class = match receiver {
                Some(receiver) => resolve_receiver_class(source, file, body_id, *receiver, query),
                None => file
                    .enclosing_type_declaration(scope)
                    .map(|declaration| (source.clone(), declaration)),
            }?;
            let (class_source, class) = receiver_class;
            let class_file = query.model_of(&class_source)?;
            let member = find_member(class_file, class, name, model::Namespace::Method)
                .into_iter()
                .next()?;
            let declaration = &class_file.declarations[member.0];
            resolve_type_reference(
                &class_source,
                class_file,
                declaration.type_ref()?,
                declaration.declaring_scope(),
                query,
            )
            .into_iter()
            .next()
        }
        model::Expression::ObjectCreation { ty, .. } => {
            resolve_type_reference(source, file, ty, scope, query)
                .into_iter()
                .next()
        }
        model::Expression::Assign { .. } | model::Expression::Literal => None,
    }
}
