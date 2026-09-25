use beans_core::{
    engine::Revision,
    model::source::{Source, SourceSpan},
};

use crate::{
    engine::JavaEngine,
    model::{File, nodes::NodeKind, references::TypeRef},
    semantics::resolution::{
        Context, JavaTypeCandidate, ResolutionFailure, TypeCandidate, resolve,
    },
};

impl JavaEngine {
    /// Finds a declaration from a Java source position in the current revision.
    pub fn goto_declaration(
        &self,
        revision: Revision,
        source: &Source,
        offset: usize,
    ) -> Option<SourceSpan> {
        let file = self.file(revision, source)?;
        let candidate = resolve_field_type_at(file, revision, source, offset)?.ok()?;
        let TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)) = candidate else {
            return None;
        };
        let declaration = self
            .file(handle.revision(), handle.source())?
            .node(handle.node_index())?
            .kind()
            .as_type()?;
        Some(SourceSpan::new(
            handle.source().clone(),
            declaration.name.as_ref()?.range(),
        ))
    }
}

/// A missing occurrence is distinct from an occurrence whose name fails resolution.
fn resolve_field_type_at(
    file: &File,
    revision: Revision,
    source: &Source,
    offset: usize,
) -> Option<Result<TypeCandidate, ResolutionFailure>> {
    // JLS §8.3: a field declares a type separately from its variable name.
    let (index, field) = file
        .iter_nodes()
        .filter_map(|entry| match entry.node.kind() {
            NodeKind::Field(field) => Some((entry.index, field)),
            _ => None,
        })
        .find(|(_, field)| {
            let TypeRef::Named { segments } = field.declared_type.value() else {
                return false;
            };
            matches!(segments.as_slice(), [segment] if segment.range().contains(offset))
        })?;

    Some(resolve(&Context::new(
        revision,
        source,
        file,
        index,
        field.declared_type.value(),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lowering::lower_into, semantics::resolution::ResolutionFailure};
    use beans_core::model::{names::Name, ranges::ByteRange};

    fn at<'a>(
        file: &'a File,
        source: &'a Source,
        text: &str,
        type_name: &str,
    ) -> Option<Result<TypeCandidate, ResolutionFailure>> {
        resolve_field_type_at(
            file,
            Revision::new(1),
            source,
            text.rfind(type_name).unwrap(),
        )
    }

    #[test]
    fn field_type_use_resolves_to_its_member_declaration() {
        let text = "class Outer { class Member {} Member field; }";
        let file = lower_into(text);
        let source = Source::uri("untitled:Example.java");
        let member_name: Name = ["Outer", "Member"].into_iter().map(str::to_owned).collect();
        let member_index = file.find_type(&member_name)[0].0;

        let Some(Ok(TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)))) =
            at(&file, &source, text, "Member")
        else {
            panic!("expected the field type to resolve to a Java declaration");
        };
        assert_eq!(handle.revision(), Revision::new(1));
        assert_eq!(handle.source(), &source);
        assert_eq!(handle.node_index(), member_index);
    }

    #[test]
    fn resolved_field_type_projects_to_its_declaration_name() {
        let text = "class Outer { class Member {} Member field; }";
        let revision = Revision::new(1);
        let source = Source::uri("untitled:Example.java");
        let mut java = JavaEngine::default();
        java.store(revision, lower_into(text), source.clone());
        let name_start = text.find("Member").unwrap();

        assert_eq!(
            java.goto_declaration(revision, &source, text.rfind("Member").unwrap()),
            Some(SourceSpan::new(
                source,
                ByteRange::new(name_start, name_start + "Member".len()),
            ))
        );
    }

    #[test]
    fn type_parameters_without_name_spans_do_not_produce_a_location() {
        let text = "class Box<T> { T field; }";
        let revision = Revision::new(1);
        let source = Source::uri("untitled:Example.java");
        let mut java = JavaEngine::default();
        java.store(revision, lower_into(text), source.clone());

        assert_eq!(
            java.goto_declaration(revision, &source, text.rfind('T').unwrap()),
            None
        );
    }

    #[test]
    fn only_the_field_type_identifier_is_an_occurrence() {
        let text = "class Outer { class Member {} Member field; int count; }";
        let file = lower_into(text);
        let source = Source::uri("untitled:Example.java");
        let at_offset = |offset| resolve_field_type_at(&file, Revision::new(1), &source, offset);

        for offset in [
            text.find("class").unwrap(),
            text.find("Member").unwrap(),
            text.rfind("Member").unwrap() + "Member".len(),
            text.find("field").unwrap(),
            text.find("int").unwrap(),
            text.find("count").unwrap(),
            text.len(),
        ] {
            assert_eq!(at_offset(offset), None, "offset {offset}");
        }
    }

    #[test]
    fn unresolved_field_type_retains_the_resolution_failure() {
        let text = "class Outer { Missing field; }";
        let file = lower_into(text);
        let source = Source::uri("untitled:Example.java");

        assert_eq!(
            at(&file, &source, text, "Missing"),
            Some(Err(ResolutionFailure::NotFound))
        );
    }
}
