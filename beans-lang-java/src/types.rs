#[derive(Debug, Clone, PartialEq)]
pub enum TypeRef {
    Simple(String),
    Qualified(String),
    Parameterized(String, Vec<TypeRef>),
    Array(Box<TypeRef>),
    Primitive(String),
    Void,
    Wildcard,
}

impl TypeRef {
    pub fn to_string_repr(&self) -> String {
        match self {
            TypeRef::Simple(s) | TypeRef::Qualified(s) | TypeRef::Primitive(s) => s.clone(),
            TypeRef::Parameterized(name, args) => {
                let args_str: Vec<String> = args.iter().map(|a| a.to_string_repr()).collect();
                format!("{}<{}>", name, args_str.join(", "))
            }
            TypeRef::Array(inner) => format!("{}[]", inner.to_string_repr()),
            TypeRef::Void => "void".to_string(),
            TypeRef::Wildcard => "?".to_string(),
        }
    }
}
