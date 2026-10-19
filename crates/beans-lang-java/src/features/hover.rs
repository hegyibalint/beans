use beans_core::{
    features::hover::{HoverProvider, HoverRequest, HoverResponse},
    location::{Location, LocationEntry},
    presentation::RichText,
};
use beans_platform_jvm::query::JvmQuery;

use crate::{
    engine::JavaLanguage,
    model::type_references::type_reference_at,
    query::JavaQuery,
    semantics::resolution::{
        JavaTypeCandidate, ResolutionFailure, TypeCandidate, query::ResolutionQuery,
    },
};

use super::{JavaFeatureContext, navigation::resolve_field_type_at};

impl<'request> HoverProvider<JavaFeatureContext<'_>, HoverRequest<'request>, HoverResponse>
    for JavaLanguage
{
    fn hover(
        &self,
        context: &JavaFeatureContext<'_>,
        request: &HoverRequest<'request>,
    ) -> Result<Option<HoverResponse>, String> {
        let java = JavaQuery::new(context.connection, context.scope);
        let Some(file) = java.file(request.source)? else {
            return Ok(None);
        };
        let Some(occurrence) = type_reference_at(&file, request.offset) else {
            return Ok(None);
        };
        let Some(written) = request
            .contents
            .get(occurrence.type_range.start()..occurrence.type_range.end())
        else {
            return Ok(None);
        };
        let definitions = ResolutionQuery::new(context.connection, context.scope);
        if let Some((_, candidate)) =
            resolve_field_type_at(&file, request.source, request.offset, Some(&definitions))
        {
            match candidate {
                Ok(candidate) => {
                    if let Some((name, location)) = resolved_type(context, candidate)? {
                        let location = display_location(&location, context.workspace_root);
                        let contents =
                            RichText::new(format!("Type: {name}\nLocated in: {location}"))
                                .with_markdown(format!(
                                    "**Type:** {}\n\n**Located in:** {}",
                                    markdown_code(&name),
                                    markdown_code(&location),
                                ));
                        return Ok(Some(HoverResponse::new(contents, occurrence.range)));
                    }
                }
                Err(ResolutionFailure::Query(error)) => return Err(error),
                Err(_) => {}
            }
        }
        Ok(Some(HoverResponse::new(
            RichText::new(written).with_markdown(markdown_code(written)),
            occurrence.range,
        )))
    }
}

fn resolved_type(
    context: &JavaFeatureContext<'_>,
    candidate: TypeCandidate,
) -> Result<Option<(String, Location)>, String> {
    let java = JavaQuery::new(context.connection, context.scope);
    match candidate {
        TypeCandidate::Jvm(location) => {
            let jvm = JvmQuery::new(context.connection, context.scope);
            Ok(jvm
                .class_name(&location)?
                .map(|name| (name.replace('/', "."), location)))
        }
        TypeCandidate::Java(JavaTypeCandidate::Declaration(handle)) => {
            let Some(file) = java.file(handle.source())? else {
                return Ok(None);
            };
            let mut names: Vec<_> = file
                .iter_ancestors(handle.node_index())
                .filter_map(|entry| entry.node.kind().as_type()?.name())
                .collect();
            names.reverse();
            let name = file
                .package_name
                .as_slice()
                .iter()
                .map(String::as_str)
                .chain(names)
                .collect::<Vec<_>>()
                .join(".");
            Ok(Some((name, handle.source().clone())))
        }
        TypeCandidate::Java(JavaTypeCandidate::TypeParameter(_)) => Ok(None),
    }
}

fn markdown_code(text: &str) -> String {
    let longest = text
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    let fence = "`".repeat(longest + 1);
    format!("{fence} {text} {fence}")
}

fn display_location(location: &Location, workspace_root: Option<&std::path::Path>) -> String {
    location
        .entries
        .iter()
        .map(|entry| match entry {
            LocationEntry::File(path) => workspace_root
                .and_then(|root| path.strip_prefix(root).ok())
                .unwrap_or(path)
                .display()
                .to_string(),
            LocationEntry::Virtual(uri) | LocationEntry::Container(uri) => uri.clone(),
        })
        .collect::<Vec<_>>()
        .join(" → ")
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
                    workspace_root: None,
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
    fn locations_are_relative_only_inside_the_workspace() {
        let root = std::path::Path::new("/project");
        for (path, expected) in [
            ("/project/src/C.java", "src/C.java"),
            ("/jdk/lib/modules", "/jdk/lib/modules"),
            ("/project-other/C.java", "/project-other/C.java"),
        ] {
            assert_eq!(
                display_location(&Location::file(path), Some(root)),
                expected
            );
        }
        let jar = Location::file("/project/lib/example.jar")
            .entry(LocationEntry::Container("p/C.class".into()));
        assert_eq!(
            display_location(&jar, Some(root)),
            "lib/example.jar → p/C.class"
        );
        assert_eq!(
            display_location(&jar, None),
            "/project/lib/example.jar → p/C.class"
        );
    }

    #[test]
    fn markdown_preserves_generics_and_backticks() {
        assert_eq!(markdown_code("Box<String>"), "` Box<String> `");
        assert_eq!(
            markdown_code("/jdk/`special`/modules"),
            "`` /jdk/`special`/modules ``"
        );
    }

    #[test]
    fn jvm_type_hover_shows_its_name_and_container_location() {
        let contents = "class C { String value; }";
        let source = source_id();
        let mut models = Models::new();
        models.store(lower_into(contents), source.clone());
        let location = Location::file("/jdk/lib/modules")
            .entry(LocationEntry::Container("java/lang/String.class".into()));
        let transaction = models.connection.transaction().unwrap();
        beans_platform_jvm::storage::store(
            &transaction,
            &beans_platform_jvm::Class {
                name: "java/lang/String".into(),
            },
            &location,
        )
        .unwrap();
        transaction.commit().unwrap();

        let response = hover(&models, &source, contents, contents.find("String").unwrap()).unwrap();

        assert_eq!(
            response.contents().plaintext(),
            "Type: java.lang.String\nLocated in: /jdk/lib/modules → java/lang/String.class"
        );
        assert_eq!(
            &contents[response.range().start()..response.range().end()],
            "String"
        );
    }

    #[test]
    fn java_type_hover_shows_its_qualified_name_and_source() {
        let contents =
            "package demo; class Box<T> { class Slot {} } class C { Box<String>.Slot value; }";
        let source = Location::file("/src/demo/C.java");
        let mut models = Models::new();
        models.store(lower_into(contents), source.clone());

        for (component, name) in [("Box<String>", "demo.Box"), ("Slot value", "demo.Box.Slot")] {
            let response = hover(
                &models,
                &source,
                contents,
                contents.find(component).unwrap(),
            )
            .unwrap();
            assert_eq!(
                response.contents().plaintext(),
                format!("Type: {name}\nLocated in: /src/demo/C.java")
            );
        }
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
            assert_eq!(response.contents().plaintext(), expected);
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

        assert_eq!(response.contents().plaintext(), "String");
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

        assert_eq!(response.contents().plaintext(), "Simple<Whatever>");
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
            assert_eq!(response.contents().plaintext(), "Qualified<Asd>.Type<Asd2>");
            assert_eq!(
                &contents[response.range().start()..response.range().end()],
                component
            );
        }
        let nested = hover(&engine, &source, contents, contents.find("Asd2").unwrap()).unwrap();
        assert_eq!(nested.contents().plaintext(), "Asd2");
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
            assert_eq!(response.contents().plaintext(), expected);
            assert_eq!(
                &contents[response.range().start()..response.range().end()],
                component
            );
        }
    }
}
