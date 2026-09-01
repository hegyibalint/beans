/// Represents a dot-separated name broken into its components
#[derive(Debug)]
pub enum NameRef {
    Simple(String),
    Qualified(Vec<String>),
}

#[derive(Debug)]
pub struct TypeRef {
    pub components: Vec<TypeRefComponent>,
}

#[derive(Debug)]
pub struct TypeRefComponent {
    pub name: String,
    pub arguments: Vec<TypeArgument>,
}

#[derive(Debug)]
pub enum TypeArgument {
    Type(TypeRef),
    Wildcard { bound: Option<WildcardBound> },
}

#[derive(Debug)]
pub enum WildcardBound {
    Extends(TypeRef),
    Super(TypeRef),
}
