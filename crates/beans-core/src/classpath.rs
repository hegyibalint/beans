use serde::{Deserialize, Serialize};

use crate::resource_filter::ResourceFilter;

/// Associates input resources with the classpath available to them.
#[derive(Debug, Serialize, Deserialize)]
pub struct Classpath {
    /// Selects the resources to which this classpath applies.
    pub inputs: Vec<ResourceFilter>,
    /// `None` is unrestricted; an empty list adds no resources beyond `inputs`.
    pub entries: Option<Vec<ResourceFilter>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serde_distinguishes_unrestricted_and_empty_classpaths() {
        for entries in [serde_json::Value::Null, json!([])] {
            let value = json!({
                "inputs": [{"File": {"root": "/project/src"}}],
                "entries": entries,
            });
            let classpath: Classpath = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(classpath).unwrap(), value);
        }
    }
}
