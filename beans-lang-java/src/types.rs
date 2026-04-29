use beans_core::PrimitiveKind;

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

    /// Convert this parser-local TypeRef to the core TypeRef.
    pub fn to_core(&self) -> beans_core::TypeRef {
        match self {
            TypeRef::Void => beans_core::TypeRef::Void,
            TypeRef::Primitive(s) => {
                match PrimitiveKind::from_str(s) {
                    Some(pk) => beans_core::TypeRef::Primitive(pk),
                    None => beans_core::TypeRef::simple(s),
                }
            }
            TypeRef::Simple(s) | TypeRef::Qualified(s) => beans_core::TypeRef::simple(s),
            TypeRef::Parameterized(name, args) => beans_core::TypeRef::parameterized(
                beans_core::TypeRef::simple(name),
                args.iter().map(|a| a.to_core()).collect(),
            ),
            TypeRef::Array(inner) => beans_core::TypeRef::array(inner.to_core()),
            TypeRef::Wildcard => beans_core::TypeRef::Wildcard { bound: None },
        }
    }
}
