use crate::{location::Location, module::Module, source::SourceSpan};

/// Matches candidate spans within the requesting resource's compilation environment.
pub struct LocationMatcher<'a> {
    modules: &'a [Module],
}

impl<'a> LocationMatcher<'a> {
    pub fn new(modules: &'a [Module]) -> Self {
        Self { modules }
    }

    /// Returns one record per admitted route; an empty result admits none.
    pub fn match_locations(
        &self,
        request: &Location,
        candidates: &[SourceSpan],
    ) -> Vec<LocationMatch> {
        // Select classpaths by ownership of the requesting resource.
        let classpaths: Vec<_> = self
            .modules
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

                for association in self
                    .modules
                    .iter()
                    .flat_map(|module| &module.artifact_sources)
                {
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
}

/// A source span and the artifact through which it is reachable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocationMatch {
    /// The declaration's source span, independent of its reachability route.
    pub origin: SourceSpan,
    /// The artifact admitting access, not necessarily where the declaration was read.
    /// `None` means no intermediate artifact was needed.
    pub via: Option<Location>,
}
