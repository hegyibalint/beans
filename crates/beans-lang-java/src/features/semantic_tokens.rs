use beans_core::{
    location::Location,
    ranges::{ByteRange, Spanned},
    revision::Revision,
};

use crate::{
    engine::JavaLanguage,
    model::{nodes::NodeKind, references::TypeRef},
    query::JavaQuery,
};

impl JavaLanguage {
    pub fn type_tokens(
        &self,
        connection: &rusqlite::Connection,
        revision: Revision,
        source: &Location,
    ) -> Result<Vec<ByteRange>, String> {
        let java = JavaQuery::new(
            connection,
            beans_core::query_scope::QueryScope::new(
                revision,
                &beans_core::classpath::Unrestricted,
            ),
        );
        let Some(file) = java.file(source)? else {
            return Ok(Vec::new());
        };
        let mut ranges = Vec::new();
        for entry in file.iter_nodes() {
            if let NodeKind::Field(field) = entry.node.kind() {
                collect(&field.declared_type, &mut ranges);
            }
        }
        ranges.sort_by_key(|range| range.start());
        Ok(ranges)
    }
}

fn collect(reference: &Spanned<TypeRef>, ranges: &mut Vec<ByteRange>) {
    match reference.value() {
        TypeRef::Named { segments } => {
            // Without resolution, a qualified prefix might be a package name.
            if let Some(last) = segments.last() {
                ranges.push(ByteRange::new(
                    last.range().start(),
                    last.range().start() + last.value().name.len(),
                ));
            }
            for segment in segments {
                for bound in &segment.value().bounds {
                    for argument in bound.types() {
                        collect(argument, ranges);
                    }
                }
            }
        }
        TypeRef::Array { element, .. } => collect(element, ranges),
        TypeRef::Primitive(_) | TypeRef::Void => {}
    }
}
