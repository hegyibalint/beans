use beans_core::{
    engine::Revision,
    model::lsp::features::hover::{HoverProvider, HoverRequest, HoverResponse},
};

use crate::engine::JavaEngine;

use crate::model::type_references::type_reference_at;

/// Shows the complete type as written at a modeled position; resolution is not wired yet.
impl HoverProvider for JavaEngine {
    fn hover(&self, revision: Revision, request: &HoverRequest<'_>) -> Option<HoverResponse> {
        let file = self.file(revision, request.source)?;
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
    use beans_core::model::source::Source;

    fn hover(
        engine: &JavaEngine,
        revision: Revision,
        source: &Source,
        contents: &str,
        offset: usize,
    ) -> Option<HoverResponse> {
        engine.hover(
            revision,
            &HoverRequest {
                source,
                contents,
                offset,
            },
        )
    }

    #[test]
    fn nested_type_argument_has_its_own_hover() {
        let contents = "class C extends Outer<String>.Inner {}";
        let mut engine = JavaEngine::default();
        let revision = Revision::new(1);
        let source = Source::uri("untitled:Example.java");
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
                &Source::class_file("C.class"),
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
        let source = Source::uri("untitled:Example.java");
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
            "Simple"
        );
    }

    #[test]
    fn qualified_type_hover_includes_every_segment_and_its_arguments() {
        let contents = "class C extends Qualified<Asd>.Type<Asd2> {}";
        let mut engine = JavaEngine::default();
        let revision = Revision::new(1);
        let source = Source::uri("untitled:Example.java");
        engine.store(revision, lower_into(contents), source.clone());

        for name in ["Qualified", "Type"] {
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
                name
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
        let source = Source::uri("untitled:Example.java");
        engine.store(revision, lower_into(contents), source.clone());

        for (name, expected) in [
            ("int", "int[]"),
            ("Simple", "Simple<Whatever>[]"),
            ("Whatever", "Whatever"),
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
                name
            );
        }
    }
}
