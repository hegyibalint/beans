pub mod class;
pub mod declaration;
pub mod imports;
pub mod scope;

/// Represents a dot-separated name broken into its components
pub struct NameRef {
    components: Vec<&str>,
}

/// Represents a whole `.java` file.
pub struct File {
    /// The package name, if exists
    package_name: Option<NameRef>,
    /// The list of imports
    imports: Vec<imports::Import>,
    /// The declared classes in the file
    classes: Vec<class::Class>,

    /// The root scope of this file
    root_scope: &scopes::Scope,
    /// Flat list of all further scopes in the file
    scopes: Map<scopes::Scope>,
}

impl File {
    pub fn new() -> File {
        todo!()
    }
}
