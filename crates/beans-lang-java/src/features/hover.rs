use beans_core::features::hover::{HoverProvider, HoverRequest, HoverResponse};

use crate::{engine::JavaEngine, model::type_references::type_reference_at};

use super::JavaFeatureContext;

impl<'request> HoverProvider<JavaFeatureContext<'_>, HoverRequest<'request>, HoverResponse>
    for JavaEngine
{
    /// Shows the complete type as written at a modeled position; resolution is not wired yet.
    fn hover(
        &self,
        context: &JavaFeatureContext<'_>,
        request: &HoverRequest<'request>,
    ) -> Option<HoverResponse> {
        let file = self.file(context.scope.revision(), request.source)?;
        let occurrence = type_reference_at(file, request.offset)?;
        Some(HoverResponse::new(
            request
                .contents
                .get(occurrence.type_range.start()..occurrence.type_range.end())?,
            occurrence.range,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::lower_into;
    use beans_core::{
        classpath::Unrestricted, origin::Origin, query_scope::QueryScope, revision::Revision,
    };
    use beans_platform_jvm::engine::JvmEngine;

    fn hover(
        engine: &JavaEngine,
        revision: Revision,
        source: &Origin,
        contents: &str,
        offset: usize,
    ) -> Option<HoverResponse> {
        engine.hover(
            &JavaFeatureContext {
                scope: QueryScope::new(revision, &Unrestricted),
                jvm: &JvmEngine::default(),
            },
            &HoverRequest {
                source,
                contents,
                offset,
            },
        )
    }

    #[test]
    fn hover_uses_the_context_revision_after_replacement_and_removal() {
        let source = Origin::uri("untitled:Example.java");
        let old = "class C extends Former {}";
        let new = "class C extends New {}";
        let mut engine = JavaEngine::default();
        engine.store(Revision::new(1), lower_into(old), source.clone());
        engine.store(Revision::new(2), lower_into(new), source.clone());
        engine.remove(Revision::new(3), source.clone());

        for (revision, contents, expected) in [
            (Revision::new(1), old, Some("Former")),
            (Revision::new(2), new, Some("New")),
            (Revision::new(3), new, None),
        ] {
            let response = hover(&engine, revision, &source, contents, 16);
            assert_eq!(response.as_ref().map(HoverResponse::contents), expected);
        }
    }

    #[test]
    fn nested_type_argument_has_its_own_hover() {
        let contents = "class C extends Outer<String>.Inner {}";
        let mut engine = JavaEngine::default();
        let revision = Revision::new(1);
        let source = Origin::uri("untitled:Example.java");
        engine.store(revision, lower_into(contents), source.clone());
        let offset = contents.find("String").unwrap();

        let response = hover(&engine, revision, &source, contents, offset).unwrap();

        assert_eq!(response.contents(), "String");
        assert_eq!(
            &contents[response.range().start()..response.range().end()],
            "String"
        );
        assert!(
            hover(
                &engine,
                revision,
                &Origin::class_file("C.class"),
                contents,
                offset
            )
            .is_none()
        );
    }

    #[test]
    fn simple_type_hover_includes_its_type_arguments() {
        let contents = "class C extends Simple<Whatever> {}";
        let mut engine = JavaEngine::default();
        let revision = Revision::new(1);
        let source = Origin::uri("untitled:Example.java");
        engine.store(revision, lower_into(contents), source.clone());

        let response = hover(
            &engine,
            revision,
            &source,
            contents,
            contents.find("Simple").unwrap(),
        )
        .unwrap();

        assert_eq!(response.contents(), "Simple<Whatever>");
        assert_eq!(
            &contents[response.range().start()..response.range().end()],
            "Simple<Whatever>"
        );
    }

    #[test]
    fn qualified_type_hover_includes_every_segment_and_its_arguments() {
        let contents = "class C extends Qualified<Asd>.Type<Asd2> {}";
        let mut engine = JavaEngine::default();
        let revision = Revision::new(1);
        let source = Origin::uri("untitled:Example.java");
        engine.store(revision, lower_into(contents), source.clone());

        for (name, component) in [("Qualified", "Qualified<Asd>"), ("Type", "Type<Asd2>")] {
            let response = hover(
                &engine,
                revision,
                &source,
                contents,
                contents.find(name).unwrap(),
            )
            .unwrap();
            assert_eq!(response.contents(), "Qualified<Asd>.Type<Asd2>");
            assert_eq!(
                &contents[response.range().start()..response.range().end()],
                component
            );
        }
        let nested = hover(
            &engine,
            revision,
            &source,
            contents,
            contents.find("Asd2").unwrap(),
        )
        .unwrap();
        assert_eq!(nested.contents(), "Asd2");
    }

    #[test]
    fn array_type_hover_includes_its_dimensions() {
        let contents = "class C { int[] counts; Simple<Whatever>[] values; }";
        let mut engine = JavaEngine::default();
        let revision = Revision::new(1);
        let source = Origin::uri("untitled:Example.java");
        engine.store(revision, lower_into(contents), source.clone());

        for (name, expected, component) in [
            ("int", "int[]", "int"),
            ("Simple", "Simple<Whatever>[]", "Simple<Whatever>"),
            ("Whatever", "Whatever", "Whatever"),
        ] {
            let response = hover(
                &engine,
                revision,
                &source,
                contents,
                contents.find(name).unwrap(),
            )
            .unwrap();
            assert_eq!(response.contents(), expected);
            assert_eq!(
                &contents[response.range().start()..response.range().end()],
                component
            );
        }
    }
}
