use crate::{location::Location, resource_filter::ResourceFilter};

#[derive(Debug)]
pub struct CompilationScopes {
    scopes: Option<Vec<CompilationScope>>,
}

impl CompilationScopes {
    pub fn new(scopes: Vec<CompilationScope>) -> Self {
        Self {
            scopes: Some(scopes),
        }
    }

    pub const fn unrestricted() -> Self {
        Self { scopes: None }
    }

    pub fn visible(&self, source: &Location, target: &Location) -> bool {
        let Some(scopes) = &self.scopes else {
            return true;
        };
        scopes
            .iter()
            .filter(|scope| scope.inputs.iter().any(|input| input.contains(source)))
            .any(|scope| scope.visible(target))
    }
}

#[derive(Debug)]
pub struct CompilationScope {
    pub inputs: Vec<ResourceFilter>,
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
    fn unrestricted_scopes_accept_all_target_locations() {
        let scopes = CompilationScopes::unrestricted();
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
    fn empty_scopes_reject_all_targets() {
        let scopes = CompilationScopes::new(vec![]);
        let source = Location::file("/consumer/Use.java");

        assert!(!scopes.visible(&source, &source));
        assert!(!scopes.visible(&source, &Location::file("/sourceset-a/Target.java")));
    }

    #[test]
    fn unrestricted_dependencies_only_apply_to_owned_sources() {
        let scopes = CompilationScopes::new(vec![CompilationScope {
            inputs: vec![ResourceFilter::file("/consumer")],
            dependencies: None,
        }]);
        let source = Location::file("/unowned/Use.java");

        assert!(scopes.visible(&Location::file("/consumer/Use.java"), &source));
        assert!(!scopes.visible(&source, &Location::file("/consumer/Target.java")));
        assert!(!scopes.visible(&source, &source));
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
}
