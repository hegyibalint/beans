use beans_core::features::hover::{HoverProvider, HoverRequest, HoverResponse};

use crate::{engine::JavaLanguage, model::type_references::type_reference_at, query::JavaQuery};

use super::JavaFeatureContext;

impl<'request> HoverProvider<JavaFeatureContext<'_>, HoverRequest<'request>, HoverResponse>
    for JavaLanguage
{
    /// Shows the complete type as written at a modeled position; resolution is not wired yet.
    fn hover(
        &self,
        context: &JavaFeatureContext<'_>,
        request: &HoverRequest<'request>,
    ) -> Result<Option<HoverResponse>, String> {
        let java = JavaQuery::new(context.connection, context.scope);
        let Some(file) = java.file(request.source)? else {
            return Ok(None);
        };
        Ok((|| {
            let occurrence = type_reference_at(&file, request.offset)?;
            Some(HoverResponse::new(
                request
                    .contents
                    .get(occurrence.type_range.start()..occurrence.type_range.end())?,
                occurrence.range,
            ))
        })())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lowering::lower_into, testing::Models};
    use beans_core::{
        classpath::Unrestricted,
        location::{Location, LocationEntry},
        query_scope::QueryScope,
    };

    fn source_id() -> Location {
        Location::new(LocationEntry::Virtual("untitled:Example.java".into()))
    }

    fn hover(
        engine: &Models,
        source: &Location,
        contents: &str,
        offset: usize,
    ) -> Option<HoverResponse> {
        JavaLanguage::new()
            .hover(
                &JavaFeatureContext {
                    connection: &engine.connection,
                    scope: QueryScope::new(&Unrestricted),
                },
                &HoverRequest {
                    source,
                    contents,
                    offset,
                },
            )
            .unwrap()
    }

    #[test]
    fn hover_follows_replacement_and_removal() {
        let source = source_id();
        let mut engine = Models::new();
        for (contents, expected) in [
            ("class C extends Former {}", "Former"),
            ("class C extends New {}", "New"),
        ] {
            engine.store(lower_into(contents), source.clone());
            let response = hover(&engine, &source, contents, 16).unwrap();
            assert_eq!(response.contents(), expected);
        }
        engine.remove(source.clone());
        assert!(hover(&engine, &source, "class C extends New {}", 16).is_none());
    }

    #[test]
    fn nested_type_argument_has_its_own_hover() {
        let contents = "class C extends Outer<String>.Inner {}";
        let mut engine = Models::new();
        let source = source_id();
        engine.store(lower_into(contents), source.clone());
        let offset = contents.find("String").unwrap();

        let response = hover(&engine, &source, contents, offset).unwrap();

        assert_eq!(response.contents(), "String");
        assert_eq!(
            &contents[response.range().start()..response.range().end()],
            "String"
        );
        assert!(hover(&engine, &Location::file("C.class"), contents, offset).is_none());
    }

    #[test]
    fn simple_type_hover_includes_its_type_arguments() {
        let contents = "class C extends Simple<Whatever> {}";
        let mut engine = Models::new();
        let source = source_id();
        engine.store(lower_into(contents), source.clone());

        let response = hover(&engine, &source, contents, contents.find("Simple").unwrap()).unwrap();

        assert_eq!(response.contents(), "Simple<Whatever>");
        assert_eq!(
            &contents[response.range().start()..response.range().end()],
            "Simple<Whatever>"
        );
    }

    #[test]
    fn qualified_type_hover_includes_every_segment_and_its_arguments() {
        let contents = "class C extends Qualified<Asd>.Type<Asd2> {}";
        let mut engine = Models::new();
        let source = source_id();
        engine.store(lower_into(contents), source.clone());

        for (name, component) in [("Qualified", "Qualified<Asd>"), ("Type", "Type<Asd2>")] {
            let response = hover(&engine, &source, contents, contents.find(name).unwrap()).unwrap();
            assert_eq!(response.contents(), "Qualified<Asd>.Type<Asd2>");
            assert_eq!(
                &contents[response.range().start()..response.range().end()],
                component
            );
        }
        let nested = hover(&engine, &source, contents, contents.find("Asd2").unwrap()).unwrap();
        assert_eq!(nested.contents(), "Asd2");
    }

    #[test]
    fn array_type_hover_includes_its_dimensions() {
        let contents = "class C { int[] counts; Simple<Whatever>[] values; }";
        let mut engine = Models::new();
        let source = source_id();
        engine.store(lower_into(contents), source.clone());

        for (name, expected, component) in [
            ("int", "int[]", "int"),
            ("Simple", "Simple<Whatever>[]", "Simple<Whatever>"),
            ("Whatever", "Whatever", "Whatever"),
        ] {
            let response = hover(&engine, &source, contents, contents.find(name).unwrap()).unwrap();
            assert_eq!(response.contents(), expected);
            assert_eq!(
                &contents[response.range().start()..response.range().end()],
                component
            );
        }
    }
}
