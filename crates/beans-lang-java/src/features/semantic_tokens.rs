use beans_core::{
    location::Location,
    ranges::{ByteRange, Spanned},
};

use crate::{
    engine::JavaLanguage,
    model::{nodes::NodeKind, references::TypeRef},
    query,
};

impl JavaLanguage {
    pub fn type_tokens(
        &self,
        source: &Location,
        connection: &rusqlite::Connection,
    ) -> Result<Vec<ByteRange>, String> {
        let Some(file) = query::file(source, connection)? else {
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
            // Type arguments distinguish a type prefix from a possible package name.
            let first_type = segments
                .iter()
                .position(|segment| !segment.value().bounds.is_empty())
                .unwrap_or(segments.len().saturating_sub(1));
            for (index, segment) in segments.iter().enumerate() {
                if index >= first_type {
                    ranges.push(ByteRange::new(
                        segment.range().start(),
                        segment.range().start() + segment.value().name.len(),
                    ));
                }
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
