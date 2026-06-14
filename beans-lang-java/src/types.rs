use beans_lang_jvm::PrimitiveKind;

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
    pub fn to_core(&self) -> beans_lang_jvm::TypeRef {
        match self {
            TypeRef::Void => beans_lang_jvm::TypeRef::Void,
            TypeRef::Primitive(s) => match PrimitiveKind::from_name(s) {
                Some(pk) => beans_lang_jvm::TypeRef::Primitive(pk),
                None => beans_lang_jvm::TypeRef::simple(s),
            },
            TypeRef::Simple(s) | TypeRef::Qualified(s) => beans_lang_jvm::TypeRef::simple(s),
            TypeRef::Parameterized(name, args) => beans_lang_jvm::TypeRef::parameterized(
                beans_lang_jvm::TypeRef::simple(name),
                args.iter().map(|a| a.to_core()).collect(),
            ),
            TypeRef::Array(inner) => beans_lang_jvm::TypeRef::array(inner.to_core()),
            TypeRef::Wildcard => beans_lang_jvm::TypeRef::Wildcard { bound: None },
        }
    }
}
