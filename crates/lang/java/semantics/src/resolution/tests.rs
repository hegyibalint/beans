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
use beans_lang_java_model::{NodeEntry, nodes::NodeKind};

fn lookup_field(
    source: &str,
    lookup: impl FnOnce(&ResolutionInstance<'_>, NodeEntry<'_>) -> ResolutionResult,
) -> ResolutionResult {
    let classpath = beans_core_model::classpath::Classpath::default();
    let files = RevisionedStorage::default();
    let query = JavaQuery::new(&files, Revision::default(), &classpath);
    lookup_field_with_query(source, &query, lookup)
}

fn lookup_field_with_query(
    source: &str,
    query: &JavaQuery<'_>,
    lookup: impl FnOnce(&ResolutionInstance<'_>, NodeEntry<'_>) -> ResolutionResult,
) -> ResolutionResult {
    let file = crate::lower_into(source);
    let (parent, field) = file
        .iter_nodes()
        .find_map(|entry| match entry.node.kind() {
            NodeKind::Field(field) if field.name == "target" => {
                Some((entry.node.parent().unwrap(), field))
            }
            _ => None,
        })
        .expect("expected target field");
    let entry = NodeEntry {
        index: parent,
        node: file.node(parent).unwrap(),
    };
    let resolver = Resolver {};
    let instance = ResolutionInstance::new(&resolver, &file, parent, &field.declared_type, query);
    lookup(&instance, entry)
}
