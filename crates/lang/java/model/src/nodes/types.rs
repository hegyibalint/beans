use crate::references::{self, TypeRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessLevel {
    Public,
    Protected,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Class,
    Enum,
    Record,
    Interface,
    AnnotationInterface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    Abstract,
    Static,
    Final,
    Sealed,
    NonSealed,
    Strictfp,
}

/// Represents a single type parameter used in a type declaration.
/// For example, `<A extends org.foo.Bar<String>>`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParameter {
    /// The name of the placeholder.
    /// Often used ones are `T`, `A`, etc...
    pub name: String,
    /// What bounds the `name` abides to
    pub bounds: Vec<references::TypeBound>,
}

#[derive(Debug)]
pub struct TypeDeclaration {
    pub name: Option<String>,
    pub type_parameters: Vec<TypeParameter>,
    pub kind: Kind,
    pub declared_superclass: Option<TypeRef>,
    pub declared_superinterfaces: Vec<TypeRef>,

    /// Plural, as nobody stops somebody writing `public public private class A`
    /// By storing multiple ones, we can diagnose and fix these cases
    pub access: Vec<AccessLevel>,
    /// Plural, as nobody stops somebody writing `abstract abstract final class A`
    /// By storing multiple ones, we can diagnose and fix these cases
    pub modifiers: Vec<Modifier>,
}

impl TypeDeclaration {
    pub fn type_parameter_named(&self, name: &str) -> Option<&TypeParameter> {
        self.type_parameters
            .iter()
            .find(|parameter| parameter.name == name)
    }

    pub fn new(kind: Kind) -> Self {
        Self {
            name: None,
            type_parameters: Vec::new(),
            kind,
            declared_superclass: None,
            declared_superinterfaces: Vec::new(),
            access: Vec::new(),
            modifiers: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Kind, TypeDeclaration, TypeParameter};

    #[test]
    fn parameter_lookup_without_parameters_returns_none() {
        let declaration = TypeDeclaration::new(Kind::Class);
        assert!(declaration.type_parameter_named("A").is_none());
    }

    #[test]
    fn parameter_lookup_returns_the_matching_stored_parameter() {
        let mut declaration = TypeDeclaration::new(Kind::Class);
        for name in ["A", "B"] {
            declaration.type_parameters.push(TypeParameter {
                name: name.into(),
                bounds: vec![],
            });
        }

        for parameter in &declaration.type_parameters {
            assert!(std::ptr::eq(
                declaration.type_parameter_named(&parameter.name).unwrap(),
                parameter
            ));
        }
    }

    #[test]
    fn parameter_lookup_requires_an_exact_name_match() {
        let mut declaration = TypeDeclaration::new(Kind::Class);
        declaration.type_parameters.push(TypeParameter {
            name: "A".into(),
            bounds: vec![],
        });

        for name in ["a", "B", ""] {
            assert!(declaration.type_parameter_named(name).is_none());
        }
    }
}
