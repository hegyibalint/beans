use crate::scopes::ScopeIndex;

pub mod declarations;
pub mod imports;
pub mod references;
pub mod scopes;

/// Represents a whole `.java` file.
#[derive(Debug)]
pub struct File {
    /// The package name, if exists
    pub package_name: Option<references::NameRef>,
    pub imports: Vec<imports::Import>,

    pub declarations: Vec<declarations::Declaration>,
    pub scopes: Vec<scopes::Scope>,
}

impl File {
    pub const ROOT_SCOPE_ID: ScopeIndex = 0;

    pub fn new() -> File {
        Self {
            package_name: None,
            imports: Vec::new(),

            declarations: Vec::new(),
            scopes: vec![
                // In scopes, we create the 0th element, the root scope
                scopes::Scope::new(),
            ],
        }
    }
}
