use beans_core::{
    features::navigation::{DefinitionProvider, DefinitionRequest, NavigationResult},
    origin::Origin,
    ranges::ByteRange,
    revision::Revision,
    source::SourceSpan,
};

use crate::{
    engine::JavaEngine,
    model::{File, nodes::NodeKind, references::TypeRef, type_references::find_in_reference},
    semantics::resolution::{
        Context, JavaTypeCandidate, ResolutionFailure, TypeCandidate,
        query::{ResolutionQuery, TypeCandidateQuery},
        resolve,
    },
};

use super::JavaFeatureContext;

impl<'request>
    DefinitionProvider<JavaFeatureContext<'_>, DefinitionRequest<'request>, NavigationResult>
    for JavaEngine
{
    /// Finds a type definition at a Java source position using the visible Java and JVM types.
    fn goto_definition(
        &self,
        context: &JavaFeatureContext<'_>,
        request: &DefinitionRequest<'request>,
    ) -> Option<NavigationResult> {
        let source = request.source;
        let offset = request.offset;
        let java = self.query(context.scope);
        let definitions = ResolutionQuery::new(&java, context.jvm);
        let file = java.file(source)?;
        if let Some(range) = type_parameter_declaration_at(file, offset) {
            return Some(NavigationResult {
                origin_range: range,
                target: SourceSpan::new(source.clone(), range),
            });
        }
        let (origin_range, candidate) = resolve_field_type_at(
            file,
            context.scope.revision(),
            source,
            offset,
            Some(&definitions),
        )?;
        let target = match candidate.ok()? {
            TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)) => {
                let declaration = self
                    .file(handle.revision(), handle.source())?
                    .node(handle.node_index())?
                    .kind()
                    .as_type()?;
                SourceSpan::new(handle.source().clone(), declaration.name.as_ref()?.range())
            }
            TypeCandidate::Java(JavaTypeCandidate::TypeParameter(handle)) => {
                let owner = handle.owner();
                let file = self.file(owner.revision(), owner.source())?;
                SourceSpan::new(owner.source().clone(), handle.parameter(file)?.name.range())
            }
            TypeCandidate::Jvm(_) => return None,
        };
        Some(NavigationResult {
            origin_range,
            target,
        })
    }
}

/// A type parameter declaration already identifies its own definition (JLS §4.4).
fn type_parameter_declaration_at(file: &File, offset: usize) -> Option<ByteRange> {
    file.iter_nodes()
        .filter_map(|entry| entry.node.kind().as_type())
        .flat_map(|declaration| &declaration.type_parameters)
        .map(|parameter| parameter.name.range())
        .find(|range| range.contains(offset) || range.end() == offset)
}

/// A missing occurrence is distinct from an occurrence whose name fails resolution.
fn resolve_field_type_at(
    file: &File,
    revision: Revision,
    source: &Origin,
    offset: usize,
    definitions: Option<&dyn TypeCandidateQuery>,
) -> Option<(ByteRange, Result<TypeCandidate, ResolutionFailure>)> {
    // JLS §8.3: a field declares a type separately from its variable name.
    let (index, prefix, origin_range) = file.iter_nodes().find_map(|entry| {
        let NodeKind::Field(field) = entry.node.kind() else {
            return None;
        };
        let reference = &field.declared_type;
        let (prefix, origin_range) = if let Some(occurrence) = find_in_reference(reference, offset)
        {
            let mut selected_reference = occurrence.reference;
            while let TypeRef::Array { element, .. } = selected_reference {
                selected_reference = element.value();
            }
            let TypeRef::Named { segments } = selected_reference else {
                return None;
            };
            let selected = segments
                .iter()
                .position(|segment| segment.range() == occurrence.range)?;
            // JLS §6.5.5.2: resolve through this component, not a later member type.
            (
                TypeRef::Named {
                    segments: segments[..=selected].to_vec(),
                },
                occurrence.range,
            )
        } else {
            // A request just after a raw final name can still select it. Dots
            // between components are not part of either occurrence.
            let TypeRef::Named { segments } = reference.value() else {
                return None;
            };
            let last = segments.last()?;
            if !last.value().bounds.is_empty() || last.range().end() != offset {
                return None;
            }
            (
                TypeRef::Named {
                    segments: segments.clone(),
                },
                last.range(),
            )
        };
        Some((entry.index, prefix, origin_range))
    })?;

    let mut ctx = Context::new(revision, source, file, index, &prefix);
    if let Some(definitions) = definitions {
        ctx = ctx.with_definitions(definitions);
    }
    Some((origin_range, resolve(&ctx)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lowering::lower_into, semantics::resolution::ResolutionFailure};
    use beans_core::{
        classpath::Unrestricted, names::Name, query_scope::QueryScope, ranges::ByteRange,
    };
    use beans_platform_jvm::engine::JvmEngine;

    fn definition_at(
        java: &JavaEngine,
        revision: Revision,
        source: &Origin,
        offset: usize,
    ) -> Option<SourceSpan> {
        java.goto_definition(
            &JavaFeatureContext {
                scope: QueryScope::new(revision, &Unrestricted),
                jvm: &JvmEngine::default(),
            },
            &DefinitionRequest { source, offset },
        )
        .map(|definition| definition.target)
    }

    fn at<'a>(
        file: &'a File,
        source: &'a Origin,
        text: &str,
        type_name: &str,
    ) -> Option<Result<TypeCandidate, ResolutionFailure>> {
        resolve_field_type_at(
            file,
            Revision::new(1),
            source,
            text.rfind(type_name).unwrap(),
            None,
        )
        .map(|(_, result)| result)
    }

    #[test]
    fn field_type_use_resolves_to_its_member_declaration() {
        let text = "class Outer { class Member {} Member field; }";
        let file = lower_into(text);
        let source = Origin::uri("untitled:Example.java");
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
        let source = Origin::uri("untitled:Example.java");
        let mut java = JavaEngine::default();
        java.store(revision, lower_into(text), source.clone());
        let name_start = text.find("Member").unwrap();

        assert_eq!(
            definition_at(&java, revision, &source, text.rfind("Member").unwrap()),
            Some(SourceSpan::new(
                source,
                ByteRange::new(name_start, name_start + "Member".len()),
            ))
        );
    }

    #[test]
    fn qualified_field_type_navigates_each_component_to_its_own_declaration() {
        let text = "class Box<T> { class Slot<U> {} } class Example { Box<String>.Slot<Integer> nested; Box<? extends Number> numbers; }";
        let revision = Revision::new(1);
        let source = Origin::uri("untitled:Example.java");
        let mut java = JavaEngine::default();
        java.store(revision, lower_into(text), source.clone());
        let box_name = text.find("class Box").unwrap() + "class ".len();
        let slot_name = text.find("class Slot").unwrap() + "class ".len();

        for (use_offset, target_offset, name) in [
            (text.find("Box<String>").unwrap(), box_name, "Box"),
            (text.find("Slot<Integer>").unwrap(), slot_name, "Slot"),
            (text.find("Box<?").unwrap(), box_name, "Box"),
        ] {
            assert_eq!(
                definition_at(&java, revision, &source, use_offset),
                Some(SourceSpan::new(
                    source.clone(),
                    ByteRange::new(target_offset, target_offset + name.len()),
                )),
                "{name} at {use_offset}"
            );
        }
        assert_eq!(
            definition_at(&java, revision, &source, text.find(".Slot").unwrap()),
            None
        );
    }

    #[test]
    fn generic_components_and_their_arguments_navigate_independently() {
        let text = "class A {} class B {} class First<T> { class Second<U> {} } class Use { First<A>.Second<B> field; }";
        let revision = Revision::new(1);
        let source = Origin::uri("untitled:Example.java");
        let mut java = JavaEngine::default();
        java.store(revision, lower_into(text), source.clone());

        let first = text.find("First<A>").unwrap();
        let second = text.find("Second<B>").unwrap();
        let declaration = |name: &str| {
            let start = text.find(&format!("class {name}")).unwrap() + "class ".len();
            Some(SourceSpan::new(
                source.clone(),
                ByteRange::new(start, start + name.len()),
            ))
        };

        for (offset, name, origin) in [
            (first, "First", "First<A>"),
            (first + "First".len(), "First", "First<A>"), // <
            (first + "First<A>".len() - 1, "First", "First<A>"), // >
            (first + "First<".len(), "A", "A"),
            (second, "Second", "Second<B>"),
            (second + "Second".len(), "Second", "Second<B>"), // <
            (second + "Second<B>".len() - 1, "Second", "Second<B>"), // >
            (second + "Second<".len(), "B", "B"),
        ] {
            let definition = java
                .goto_definition(
                    &JavaFeatureContext {
                        scope: QueryScope::new(revision, &Unrestricted),
                        jvm: &JvmEngine::default(),
                    },
                    &DefinitionRequest {
                        source: &source,
                        offset,
                    },
                )
                .unwrap();
            assert_eq!(Some(definition.target), declaration(name));
            assert_eq!(
                &text[definition.origin_range.start()..definition.origin_range.end()],
                origin
            );
        }
        assert_eq!(
            definition_at(&java, revision, &source, first + "First<A>".len()),
            None
        );
    }

    #[test]
    fn field_type_parameter_navigates_to_its_declaration_name() {
        // JLS §6.3 and §6.5.5.1: the class type parameter is in scope in its body.
        let text = "class Example<T extends Labelled> { T label; }";
        let revision = Revision::new(1);
        let source = Origin::uri("untitled:Example.java");
        let mut java = JavaEngine::default();
        java.store(revision, lower_into(text), source.clone());
        let declaration = text.find("<T").unwrap() + 1;
        let use_offset = text.find("T label").unwrap();
        let file = java.file(revision, &source).unwrap();
        assert!(matches!(
            resolve_field_type_at(file, revision, &source, use_offset, None),
            Some((
                _,
                Ok(TypeCandidate::Java(JavaTypeCandidate::TypeParameter(_)))
            ))
        ));

        assert_eq!(
            definition_at(&java, revision, &source, use_offset),
            Some(SourceSpan::new(
                source,
                ByteRange::new(declaration, declaration + 1),
            ))
        );
    }

    #[test]
    fn type_parameter_declaration_names_navigate_to_themselves() {
        let text = "class Box<T, U extends Labelled> { U label; }";
        let revision = Revision::new(1);
        let source = Origin::uri("untitled:Example.java");
        let mut java = JavaEngine::default();
        java.store(revision, lower_into(text), source.clone());

        for offset in [
            text.find("<T").unwrap() + 1,
            text.find("U extends").unwrap(),
        ] {
            assert_eq!(
                definition_at(&java, revision, &source, offset),
                Some(SourceSpan::new(
                    source.clone(),
                    ByteRange::new(offset, offset + 1)
                ))
            );
        }
        assert_eq!(
            definition_at(&java, revision, &source, text.find("<T").unwrap() + 2),
            Some(SourceSpan::new(
                source.clone(),
                ByteRange::new(text.find("<T").unwrap() + 1, text.find("<T").unwrap() + 2)
            ))
        );
        for offset in [
            text.find("<T").unwrap(),
            text.find("U extends").unwrap() + 2,
        ] {
            assert_eq!(definition_at(&java, revision, &source, offset), None);
        }
    }

    #[test]
    fn field_type_end_positions_navigate_when_entered_from_the_right() {
        let text = "class Box<T> { class Slot<U> {} T value; Box<String>.Slot<Integer> nested; }";
        let revision = Revision::new(1);
        let source = Origin::uri("untitled:Example.java");
        let mut java = JavaEngine::default();
        java.store(revision, lower_into(text), source.clone());

        for (use_start, target_start, name) in [
            (
                text.find("T value").unwrap(),
                text.find("<T").unwrap() + 1,
                "T",
            ),
            (
                text.find("Box<String>").unwrap(),
                text.find("class Box").unwrap() + 6,
                "Box",
            ),
            (
                text.find("Slot<Integer>").unwrap(),
                text.find("class Slot").unwrap() + 6,
                "Slot",
            ),
        ] {
            assert_eq!(
                definition_at(&java, revision, &source, use_start + name.len()),
                Some(SourceSpan::new(
                    source.clone(),
                    ByteRange::new(target_start, target_start + name.len())
                )),
                "end of {name}"
            );
        }
    }

    #[test]
    fn unrelated_positions_do_not_resolve_to_field_types() {
        let text = "class Outer { class Member {} Member field; int count; }";
        let file = lower_into(text);
        let source = Origin::uri("untitled:Example.java");
        let at_offset =
            |offset| resolve_field_type_at(&file, Revision::new(1), &source, offset, None);

        for offset in [
            text.find("class").unwrap(),
            text.find("Member").unwrap(),
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
        let source = Origin::uri("untitled:Example.java");

        assert_eq!(
            at(&file, &source, text, "Missing"),
            Some(Err(ResolutionFailure::NotFound))
        );
    }
}
