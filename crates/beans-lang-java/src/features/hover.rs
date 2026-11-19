use beans_core::{
    features::hover::{HoverProvider, HoverRequest, HoverResponse},
    location::{Location, LocationEntry},
    presentation::RichText,
};
use beans_platform_jvm::query as jvm_query;

use crate::{
    engine::JavaLanguage,
    model::type_references::type_reference_at,
    query,
    semantics::resolution::{ResolutionFailure, TypeLocator},
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
        let Some(file) = query::file(request.source, context.connection)? else {
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
        if let Some((_, candidate)) = resolve_field_type_at(
            &file,
            request.source,
            request.offset,
            context.location_matcher,
            context.connection,
        ) {
            match candidate {
                Ok(candidate) => {
                    if let Some(name) = resolved_type_name(context, candidate.locator)? {
                        let location = display_location(
                            &candidate.provenance.origin.resource,
                            context.workspace_root,
                        );
                        let mut plaintext = format!("Type: {name}\nLocated in: {location}");
                        let mut markdown = format!(
                            "**Type:** {}\n\n**Located in:** {}",
                            markdown_code(&name),
                            markdown_code(&location),
                        );
                        if let Some(via) = candidate.provenance.via {
                            let via = display_location(&via, context.workspace_root);
                            plaintext.push_str(&format!("\nVia: {via}"));
                            markdown.push_str(&format!("\n\n**Via:** {}", markdown_code(&via)));
                        }
                        let contents = RichText::new(plaintext).with_markdown(markdown);
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

fn resolved_type_name(
    context: &JavaFeatureContext<'_>,
    locator: TypeLocator,
) -> Result<Option<String>, String> {
    match locator {
        TypeLocator::Jvm(location) => Ok(jvm_query::class_name(&location, context.connection)?
            .map(|name| name.replace('/', "."))),
        TypeLocator::Declaration(handle) => {
            let Some(file) = query::file(handle.source(), context.connection)? else {
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
            Ok(Some(name))
        }
        TypeLocator::TypeParameter(_) => Ok(None),
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
