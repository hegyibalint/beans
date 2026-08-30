//! JLS §8.2 and §9.2: the members a type has that it did not declare.
//!
//! Every other module here answers a question about one `model::File`. This one
//! cannot: a superclass is usually in another file, so a declaration stops being
//! an index and becomes an index paired with the source it indexes into. That
//! pairing is the whole reason this is its own module rather than three more
//! lines in `methods.rs`.

use std::collections::HashSet;

use beans_platform_jvm as jvm;

use crate::{model, query::Query};

use super::{Wanted, methods::members_of, types::resolve_type_reference};

/// §15.12.1's "class to search", flattened for a type that inherits.
///
/// §8.2 defines a class's members recursively: the ones it declares, then the
/// ones it inherits from its direct superclass — which brings its own inherited
/// members with it — then the same from each direct superinterface. Flattening
/// that is depth first with the superclass before the interfaces, which is the
/// order `TypeDeclaration::supertypes` already states.
///
/// The type itself is the first entry, because §8.2 makes its own declarations
/// the nearest ones and a caller wants one list rather than two.
///
/// A type is visited once. §8.1.5 lets two superinterfaces share an ancestor,
/// and a cycle is illegal (§8.1.4) but entirely writable while someone is
/// typing, so one set stops the diamond and the loop together.
///
/// The walk ends at anything we did not parse. `resolve_type_reference` drops a
/// `TypeTarget::Compiled`, so a supertype that is a class file contributes
/// nothing — and so does the implicit `Object` of §8.1.4, which is not written
/// in the source and therefore not in the model at all. That is why `toString`
/// is still offered nowhere, and it is the compiled-members entry in `TODO.md`
/// rather than this one.
pub(crate) fn types_to_search(
    source: &jvm::model::Source,
    declaration: model::DeclarationId,
    query: &Query,
) -> Vec<(jvm::model::Source, model::DeclarationId)> {
    let mut order = Vec::new();
    let mut visited = HashSet::new();
    walk(source, declaration, query, &mut order, &mut visited);
    order
}

fn walk(
    source: &jvm::model::Source,
    declaration: model::DeclarationId,
    query: &Query,
    order: &mut Vec<(jvm::model::Source, model::DeclarationId)>,
    visited: &mut HashSet<(jvm::model::Source, model::DeclarationId)>,
) {
    if !visited.insert((source.clone(), declaration)) {
        return;
    }
    order.push((source.clone(), declaration));

    let Some(file) = query.model_of(source) else {
        return;
    };
    let model::Declaration::Type(ty) = &file.declarations[declaration.0] else {
        return;
    };

    // §8.1.4 and §9.1.3 write a supertype's name beside the class and not
    // inside it, so `declaring_scope` is where the name is looked up — the body
    // scope would let a member type shadow what the clause meant.
    let supertypes: Vec<_> = ty
        .supertypes()
        .flat_map(|(_, type_ref)| {
            resolve_type_reference(source, file, type_ref, ty.declaring_scope, query)
        })
        .collect();

    for (super_source, super_declaration) in supertypes {
        walk(&super_source, super_declaration, query, order, visited);
    }
}

/// Every member of a type in one namespace, inherited ones included, nearest
/// first.
///
/// Nearest first is what makes the list usable rather than merely complete:
/// §8.3 has a field redeclaration *hide* the one above it and §8.5 says the
/// same of a member type, so the first entry for a name is the one that wins
/// and a caller can drop the rest.
///
/// §6.6.1 is not applied here. §8.2 does exclude a private member from what a
/// subclass inherits, but that is the same question the caller already asks of
/// every member it shows, and asking it in one place beats asking it in two
/// with different answers. So the walk reports what is declared.
pub(crate) fn members_in_hierarchy(
    source: &jvm::model::Source,
    declaration: model::DeclarationId,
    namespace: model::Namespace,
    wanted: Wanted<'_>,
    query: &Query,
) -> Vec<(jvm::model::Source, model::DeclarationId)> {
    types_to_search(source, declaration, query)
        .into_iter()
        .flat_map(|(type_source, type_declaration)| {
            let Some(file) = query.model_of(&type_source) else {
                return Vec::new();
            };
            members_of(file, type_declaration, namespace, wanted)
                .map(|member| (type_source.clone(), member))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// What a name reaches through a type, hiding applied.
///
/// Only the nearest type to declare the name contributes, which is §8.3 for a
/// field and §8.5 for a member type. Methods are the case the rule cannot
/// state: an inherited method of the same name is overridden (§8.4.8) or
/// overloaded (§8.4.9) rather than hidden, and telling those apart needs
/// §8.4.2's signature with both parameter lists resolved. Nearest wins is the
/// approximation, and it is the right one for navigation — it lands where a
/// reader looking for the declaration would stop.
pub(crate) fn find_member(
    source: &jvm::model::Source,
    declaration: model::DeclarationId,
    name: &model::Identifier,
    namespace: model::Namespace,
    query: &Query,
) -> Vec<(jvm::model::Source, model::DeclarationId)> {
    for (type_source, type_declaration) in types_to_search(source, declaration, query) {
        let Some(file) = query.model_of(&type_source) else {
            continue;
        };
        let found: Vec<_> = members_of(
            file,
            type_declaration,
            namespace,
            Wanted::Exactly(&name.text),
        )
        .map(|member| (type_source.clone(), member))
        .collect();

        if !found.is_empty() {
            return found;
        }
    }

    Vec::new()
}
