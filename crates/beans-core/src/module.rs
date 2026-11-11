use serde::{Deserialize, Serialize};

use crate::{artifact_sources::ArtifactSources, classpath::Classpath};

/// A module's classpaths and artifact-to-source associations.
#[derive(Debug, Serialize, Deserialize)]
pub struct Module {
    pub classpaths: Vec<Classpath>,
    pub artifact_sources: Vec<ArtifactSources>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serde_preserves_classpaths_and_artifact_source_associations() {
        let value = json!({
            "classpaths": [{
                "inputs": [{"File": {"root": "/project/engine/src/Use.java"}}],
                "entries": [{"File": {"root": "/project/api/api.jar"}}],
            }],
            "artifact_sources": [{
                "artifacts": [
                    [{"File": "/project/api/classes"}],
                    [{"File": "/project/api/api.jar"}],
                ],
                "sources": [
                    {"File": {"root": "/project/api/src/Target.java"}},
                    {"File": {"root": "/project/api/api-sources.jar"}},
                ],
            }],
        });
        let module: Module = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(module).unwrap(), value);
    }
}
