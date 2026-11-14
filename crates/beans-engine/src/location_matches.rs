use beans_core::{
    location::Location, location_match::LocationMatch, module::Module, source::SourceSpan,
};

use crate::Engine;

impl Engine {
    /// Matches candidate spans against the request's classpaths, preserving artifact routes.
    pub fn match_locations(
        &self,
        request: &Location,
        candidates: &[SourceSpan],
    ) -> Result<Vec<LocationMatch>, String> {
        let modules = self.modules()?;
        Ok(match_locations(&modules, request, candidates))
    }
}

/// Binds a language request to the metadata from its existing database snapshot.
pub(super) fn matcher(
    modules: &[Module],
) -> impl Fn(&Location, &[SourceSpan]) -> Result<Vec<LocationMatch>, String> + '_ {
    move |request, candidates| Ok(match_locations(modules, request, candidates))
}

fn match_locations(
    modules: &[Module],
    request: &Location,
    candidates: &[SourceSpan],
) -> Vec<LocationMatch> {
    // Select classpaths by ownership of the requesting resource.
    let classpaths: Vec<_> = modules
        .iter()
        .flat_map(|module| &module.classpaths)
        .filter(|classpath| classpath.inputs.iter().any(|input| input.contains(request)))
        .collect();

    // Collect every admitted artifact path without widening through producer classpaths.
    let mut matches = Vec::new();
    for candidate in candidates {
        let mut direct = classpaths.is_empty();
        let mut artifacts = Vec::new();
        for classpath in &classpaths {
            let Some(entries) = &classpath.entries else {
                direct = true;
                continue;
            };
            if classpath
                .inputs
                .iter()
                .chain(entries)
                .any(|filter| filter.contains(&candidate.resource))
            {
                direct = true;
            }

            for association in modules.iter().flat_map(|module| &module.artifact_sources) {
                if !association
                    .sources
                    .iter()
                    .any(|filter| filter.contains(&candidate.resource))
                {
                    continue;
                }
                for artifact in &association.artifacts {
                    if entries.iter().any(|entry| entry.contains(artifact))
                        && !artifacts.contains(artifact)
                    {
                        artifacts.push(artifact.clone());
                    }
                }
            }
        }

        // Preserve direct access separately from each intermediate artifact.
        if direct {
            matches.push(LocationMatch {
                origin: candidate.clone(),
                via: None,
            });
        }
        for artifact in artifacts {
            matches.push(LocationMatch {
                origin: candidate.clone(),
                via: Some(artifact),
            });
        }
    }
    matches
}
