mod imported_type;
mod member_path;
mod scopes;
mod simple_type;
mod simple_type_declaration;
mod simple_type_parameter;
mod stages;

use super::{ResolutionInstance, ResolutionResult, Resolver};
use crate::query::JavaQuery;
use beans_core_engine::{Revision, storage::RevisionedStorage};
use beans_lang_java_model::{declarations::Declaration, ScopeEntry};

fn lookup_field(
    source: &str,
    lookup: impl FnOnce(&ResolutionInstance<'_>, ScopeEntry<'_>) -> ResolutionResult,
) -> ResolutionResult {
    let classpath = beans_core_model::classpath::Classpath::default();
    let files = RevisionedStorage::default();
    let query = JavaQuery::new(&files, Revision::default(), &classpath);
    lookup_field_with_query(source, &query, lookup)
}

fn lookup_field_with_query(
    source: &str,
    query: &JavaQuery<'_>,
    lookup: impl FnOnce(&ResolutionInstance<'_>, ScopeEntry<'_>) -> ResolutionResult,
) -> ResolutionResult {
    let file = crate::lower_into(source);
    let (entry, field) = file
        .iter_declarations()
        .find_map(|entry| match entry.declaration {
            Declaration::Field(field) if field.name == "target" => Some((
                ScopeEntry {
                    scope_index: entry.scope_index,
                    scope: entry.scope,
                },
                field,
            )),
            _ => None,
        })
        .expect("expected target field");
    let resolver = Resolver {};
    let instance = ResolutionInstance::new(
        &resolver,
        &file,
        entry.scope_index,
        &field.declared_type,
        query,
    );

    lookup(&instance, entry)
}
