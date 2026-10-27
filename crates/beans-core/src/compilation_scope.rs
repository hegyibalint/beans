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

    pub fn visible(&self, _source: &Location, _target: &Location) -> bool {
        if self.scopes.is_none() {
            return true;
        }
        todo!("select applicable compilation scopes and check target visibility")
    }
}

#[derive(Debug)]
pub struct CompilationScope {
    pub inputs: Vec<ResourceFilter>,
    pub dependencies: Option<Vec<ResourceFilter>>,
}

impl CompilationScope {
    pub fn visible(&self, _target: &Location) -> bool {
        todo!("unify source and class resource matching")
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
}
