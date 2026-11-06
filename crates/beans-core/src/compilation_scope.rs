use serde::{Deserialize, Serialize};

use crate::{location::Location, resource_filter::ResourceFilter};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CompilationScopes {
    scopes: Vec<CompilationScope>,
}

impl CompilationScopes {
    pub const fn new(scopes: Vec<CompilationScope>) -> Self {
        Self { scopes }
    }

    pub fn visible(&self, source: &Location, target: &Location) -> bool {
        // Apply visibility from scopes that own the source.
        let mut matched = false;
        for scope in &self.scopes {
            if !scope.inputs.iter().any(|input| input.contains(source)) {
                continue;
            }
            matched = true;
            if scope.visible(target) {
                return true;
            }
        }

        // Sources outside all declared inputs remain unrestricted.
        !matched
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompilationScope {
    pub inputs: Vec<ResourceFilter>,
    /// `None` is unrestricted; an empty list adds no resources beyond `inputs`.
    pub dependencies: Option<Vec<ResourceFilter>>,
}

impl CompilationScope {
    pub fn visible(&self, target: &Location) -> bool {
        let Some(dependencies) = &self.dependencies else {
            return true;
        };
        self.inputs
            .iter()
            .chain(dependencies)
            .any(|filter| filter.contains(target))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::location::LocationEntry;

    #[test]
    fn empty_scopes_accept_all_target_locations() {
        let scopes = CompilationScopes::default();
        let source = Location::file("/src/Use.java");
        for target in [
            Location::file("/other/Example.java"),
            Location::file("/library.jar")
                .entry(LocationEntry::Container("p/Example.class".into())),
            Location::new(LocationEntry::Virtual("untitled:Example.java".into())),
        ] {
            assert!(scopes.visible(&source, &target));
        }
    }

    #[test]
    fn scope_without_dependencies_sees_only_its_inputs() {
        let scope = CompilationScope {
            inputs: vec![
                ResourceFilter::file("/consumer"),
                ResourceFilter::file("/classes"),
            ],
            dependencies: Some(vec![]),
        };

        for path in ["/consumer/Use.java", "/classes/Target.class"] {
            assert!(scope.visible(&Location::file(path)));
        }
        assert!(!scope.visible(&Location::file("/sourceset-a/Target.java")));
    }

    #[test]
    fn scope_sees_its_inputs_and_matching_dependencies() {
        let scope = CompilationScope {
            inputs: vec![ResourceFilter::file("/consumer")],
            dependencies: Some(vec![
                ResourceFilter::file("/sourceset-a"),
                ResourceFilter::file("/library.jar"),
            ]),
        };

        for target in [
            Location::file("/consumer/Use.java"),
            Location::file("/sourceset-a/Target.java"),
            Location::file("/library.jar").entry(LocationEntry::Container("p/Target.class".into())),
        ] {
            assert!(scope.visible(&target));
        }
        assert!(!scope.visible(&Location::file("/sourceset-b/Target.java")));
    }

    #[test]
    fn scope_with_unrestricted_dependencies_sees_all_targets() {
        let scope = CompilationScope {
            inputs: vec![],
            dependencies: None,
        };

        for target in [
            Location::file("/other/Target.java"),
            Location::file("/library.jar").entry(LocationEntry::Container("p/Target.class".into())),
            Location::new(LocationEntry::Virtual("untitled:Target.java".into())),
        ] {
            assert!(scope.visible(&target));
        }
    }

    #[test]
    fn unmatched_sources_are_unrestricted() {
        let scopes = CompilationScopes::new(vec![CompilationScope {
            inputs: vec![ResourceFilter::file("/consumer")],
            dependencies: Some(vec![]),
        }]);
        let source = Location::file("/unowned/Use.java");

        assert!(scopes.visible(&source, &source));
        assert!(scopes.visible(&source, &Location::file("/consumer/Target.java")));
        assert!(scopes.visible(&source, &Location::file("/other/Target.java")));
        assert!(!scopes.visible(&Location::file("/consumer/Use.java"), &source));
    }

    #[test]
    fn non_matching_unrestricted_scopes_do_not_bypass_matching_restrictions() {
        let scopes = CompilationScopes::new(vec![
            CompilationScope {
                inputs: vec![ResourceFilter::file("/consumer")],
                dependencies: Some(vec![]),
            },
            CompilationScope {
                inputs: vec![ResourceFilter::file("/other")],
                dependencies: None,
            },
            CompilationScope {
                inputs: vec![],
                dependencies: None,
            },
        ]);
        let source = Location::file("/consumer/Use.java");
        let other = Location::file("/other/Target.java");

        assert!(!scopes.visible(&source, &other));
        assert!(scopes.visible(&other, &source));
    }

    #[test]
    fn a_matching_unrestricted_scope_overrides_matching_restrictions() {
        let scopes = CompilationScopes::new(vec![
            CompilationScope {
                inputs: vec![ResourceFilter::file("/consumer")],
                dependencies: Some(vec![]),
            },
            CompilationScope {
                inputs: vec![ResourceFilter::file("/consumer/sub")],
                dependencies: None,
            },
        ]);
        let target = Location::file("/other/Target.java");

        assert!(scopes.visible(&Location::file("/consumer/sub/Use.java"), &target));
        assert!(!scopes.visible(&Location::file("/consumer/Use.java"), &target));
    }

    #[test]
    fn overlapping_scopes_combine_visibility_only_when_they_own_the_source() {
        let scopes = CompilationScopes::new(vec![
            CompilationScope {
                inputs: vec![ResourceFilter::file("/consumer")],
                dependencies: Some(vec![ResourceFilter::file("/sourceset-a")]),
            },
            CompilationScope {
                inputs: vec![ResourceFilter::file("/consumer/sub")],
                dependencies: Some(vec![ResourceFilter::file("/sourceset-b")]),
            },
        ]);
        let source = Location::file("/consumer/sub/Use.java");

        for path in ["/sourceset-a/Target.java", "/sourceset-b/Target.java"] {
            assert!(scopes.visible(&source, &Location::file(path)));
        }
        assert!(!scopes.visible(&source, &Location::file("/other/Target.java")));
        assert!(!scopes.visible(
            &Location::file("/consumer/Use.java"),
            &Location::file("/sourceset-b/Target.java"),
        ));
    }

    #[test]
    fn dependency_membership_does_not_select_a_scope() {
        let scopes = CompilationScopes::new(vec![
            CompilationScope {
                inputs: vec![ResourceFilter::file("/sourceset-a")],
                dependencies: Some(vec![]),
            },
            CompilationScope {
                inputs: vec![ResourceFilter::file("/sourceset-b")],
                dependencies: Some(vec![ResourceFilter::file("/sourceset-a")]),
            },
        ]);

        assert!(!scopes.visible(
            &Location::file("/sourceset-a/Use.java"),
            &Location::file("/sourceset-b/Target.java"),
        ));
        assert!(scopes.visible(
            &Location::file("/sourceset-b/Use.java"),
            &Location::file("/sourceset-a/Target.java"),
        ));
    }

    #[test]
    fn container_entries_can_belong_to_scope_inputs() {
        let scopes = CompilationScopes::new(vec![CompilationScope {
            inputs: vec![ResourceFilter::file("/library.jar")],
            dependencies: Some(vec![]),
        }]);
        let source =
            Location::file("/library.jar").entry(LocationEntry::Container("p/Use.java".into()));
        let target =
            Location::file("/library.jar").entry(LocationEntry::Container("p/Target.class".into()));

        assert!(scopes.visible(&source, &target));
        assert!(!scopes.visible(&source, &Location::file("/other/Target.java")));
    }

    #[test]
    fn serde_preserves_empty_scopes_as_an_unrestricted_collection() {
        let expected = serde_json::json!({"scopes": []});
        assert_eq!(
            serde_json::to_value(CompilationScopes::default()).unwrap(),
            expected
        );

        let decoded: CompilationScopes = serde_json::from_value(expected).unwrap();
        let source = Location::file("/src/Use.java");
        assert!(decoded.visible(&source, &Location::file("/other/Target.java")));
    }

    #[test]
    fn serde_requires_a_scope_array() {
        for json in [serde_json::json!({"scopes": null}), serde_json::json!({})] {
            assert!(serde_json::from_value::<CompilationScopes>(json).is_err());
        }
    }

    #[test]
    fn serde_distinguishes_unrestricted_and_empty_dependencies() {
        let source = Location::file("/consumer/Use.java");
        let target = Location::file("/other/Target.java");
        for (dependencies, expected, visible) in [
            (None, serde_json::Value::Null, true),
            (Some(vec![]), serde_json::json!([]), false),
        ] {
            let scopes = CompilationScopes::new(vec![CompilationScope {
                inputs: vec![ResourceFilter::file("/consumer")],
                dependencies,
            }]);
            let expected = serde_json::json!({"scopes": [{
                "inputs": [{"File": {"root": "/consumer"}}],
                "dependencies": expected,
            }]});
            assert_eq!(serde_json::to_value(scopes).unwrap(), expected);

            let decoded: CompilationScopes = serde_json::from_value(expected).unwrap();
            assert!(decoded.visible(&source, &source));
            assert_eq!(decoded.visible(&source, &target), visible);
        }
    }

    #[test]
    fn serde_preserves_scope_and_filter_order() {
        // Build an import result with ordered filters and an empty scope.
        let scopes = CompilationScopes::new(vec![
            CompilationScope {
                inputs: vec![
                    ResourceFilter::file("/project/src"),
                    ResourceFilter::file("/project/generated"),
                ],
                dependencies: Some(vec![
                    ResourceFilter::file("/libs/b.jar"),
                    ResourceFilter::file("/libs/a.jar"),
                ]),
            },
            CompilationScope {
                inputs: vec![],
                dependencies: Some(vec![]),
            },
        ]);

        // Check the shared JSON representation.
        let expected = serde_json::json!({"scopes": [
            {
                "inputs": [
                    {"File": {"root": "/project/src"}},
                    {"File": {"root": "/project/generated"}},
                ],
                "dependencies": [
                    {"File": {"root": "/libs/b.jar"}},
                    {"File": {"root": "/libs/a.jar"}},
                ],
            },
            {"inputs": [], "dependencies": []},
        ]});
        assert_eq!(serde_json::to_value(scopes).unwrap(), expected);

        // Decode the same JSON without changing its structure.
        let decoded: CompilationScopes = serde_json::from_value(expected.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), expected);
    }
}
