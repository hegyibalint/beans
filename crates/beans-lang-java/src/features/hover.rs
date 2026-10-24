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
}
