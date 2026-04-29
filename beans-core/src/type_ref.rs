//! Structured types at the JVM level.
//!
//! `TypeRef` is the JVM-layer type representation — the lingua franca that
//! every language module projects into for cross-language interop. It models
//! the JVM's view of types: classes, interfaces, primitives, parameterized
//! types, wildcards, arrays. Per ADR-0004, this is intentionally the
//! *intersection* of what all five JVM languages need to talk about each
//! other, not a superset of any one language.
//!
//! Language-specific type features that don't fit the JVM model — Kotlin's
//! nullability (`String?`), Scala's union types (`A | B`), match types,
//! path-dependent types, and so on — are *not* in `TypeRef`. They live in
//! the per-language rich model and only flatten into `TypeRef` at the
//! JVM-projection boundary, possibly with information loss. A Kotlin
//! `String?` projects to `TypeRef::Simple { name: "java.lang.String" }`
//! plus a nullability bit on the JVM-projection node, not into a TypeRef
//! variant.
//!
//! What `TypeRef` enables (cross-file semantic analysis that string-based
//! types could not support):
//! - Erasure (`erasure()`, JLS §4.6) — collapse parameterized types and
//!   type variables to their JVM bytecode shape.
//! - Substitution (`substitute()`) — replace type variables with concrete
//!   arguments through inheritance chains.
//! - Subtype checking, when paired with the supertype graph.
//! - Stable structural equality across files.

/// A structured JVM-level type reference.
///
/// See the module doc for what's deliberately *not* in this enum (nullability,
/// union types, etc. — those belong in language-specific models).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeRef {
    /// `void`
    Void,

    /// Primitives: `int`, `boolean`, `double`, etc.
    Primitive(PrimitiveKind),

    /// A simple named type, possibly unresolved: `String`, `com.example.Foo`
    Simple {
        /// The name as written — may be simple ("String") or qualified ("java.util.List").
        /// Resolution turns simple names into FQNs.
        name: String,
    },

    /// A parameterized type: `List<String>`, `Map<K, V>`, `Comparable<? super T>`
    Parameterized {
        /// The raw type (e.g., "java.util.List")
        raw: Box<TypeRef>,
        /// Type arguments
        args: Vec<TypeRef>,
    },

    /// A reference to a type variable declared by a class or method: `T`, `E`
    TypeVariable {
        name: String,
    },

    /// A wildcard: `?`, `? extends Foo`, `? super Bar`
    Wildcard {
        bound: Option<WildcardBound>,
    },

    /// An array type: `int[]`, `String[][]`
    Array {
        element: Box<TypeRef>,
    },

    /// An intersection type (internal, for type parameter bounds): `Serializable & Comparable<T>`
    Intersection {
        types: Vec<TypeRef>,
    },

    /// Unresolved / error sentinel — the parser couldn't determine the type.
    /// Propagates gracefully through analysis without crashing.
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveKind {
    Boolean,
    Byte,
    Short,
    Int,
    Long,
    Float,
    Double,
    Char,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WildcardBound {
    /// `? extends Foo`
    Extends(Box<TypeRef>),
    /// `? super Foo`
    Super(Box<TypeRef>),
}

/// A type parameter declaration: `T`, `T extends Comparable<T>`, `T extends A & B`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeParam {
    pub name: String,
    /// Upper bounds. Empty means implicit `Object` bound.
    /// Multiple bounds = intersection: `T extends A & B`
    pub bounds: Vec<TypeRef>,
}

// ---- Comparison with str (for ergonomic test assertions) ----

impl PartialEq<str> for TypeRef {
    fn eq(&self, other: &str) -> bool {
        self.to_string() == other
    }
}

impl PartialEq<&str> for TypeRef {
    fn eq(&self, other: &&str) -> bool {
        self.to_string() == *other
    }
}

// ---- Display ----

impl std::fmt::Display for TypeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeRef::Void => write!(f, "void"),
            TypeRef::Primitive(p) => write!(f, "{p}"),
            TypeRef::Simple { name } => write!(f, "{name}"),
            TypeRef::Parameterized { raw, args } => {
                write!(f, "{raw}<")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{arg}")?;
                }
                write!(f, ">")
            }
            TypeRef::TypeVariable { name } => write!(f, "{name}"),
            TypeRef::Wildcard { bound: None } => write!(f, "?"),
            TypeRef::Wildcard { bound: Some(WildcardBound::Extends(t)) } => {
                write!(f, "? extends {t}")
            }
            TypeRef::Wildcard { bound: Some(WildcardBound::Super(t)) } => {
                write!(f, "? super {t}")
            }
            TypeRef::Array { element } => write!(f, "{element}[]"),
            TypeRef::Intersection { types } => {
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, " & ")?;
                    }
                    write!(f, "{t}")?;
                }
                Ok(())
            }
            TypeRef::Unknown => write!(f, "<unknown>"),
        }
    }
}

impl std::fmt::Display for PrimitiveKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimitiveKind::Boolean => write!(f, "boolean"),
            PrimitiveKind::Byte => write!(f, "byte"),
            PrimitiveKind::Short => write!(f, "short"),
            PrimitiveKind::Int => write!(f, "int"),
            PrimitiveKind::Long => write!(f, "long"),
            PrimitiveKind::Float => write!(f, "float"),
            PrimitiveKind::Double => write!(f, "double"),
            PrimitiveKind::Char => write!(f, "char"),
        }
    }
}

// ---- Constructors ----

impl TypeRef {
    /// Convenience: `TypeRef::simple("java.util.List")`
    pub fn simple(name: impl Into<String>) -> Self {
        TypeRef::Simple { name: name.into() }
    }

    /// Convenience: `TypeRef::type_var("T")`
    pub fn type_var(name: impl Into<String>) -> Self {
        TypeRef::TypeVariable { name: name.into() }
    }

    /// Convenience: `TypeRef::array(TypeRef::Primitive(PrimitiveKind::Int))`
    pub fn array(element: TypeRef) -> Self {
        TypeRef::Array { element: Box::new(element) }
    }

    /// Convenience: `TypeRef::parameterized(TypeRef::simple("List"), vec![TypeRef::simple("String")])`
    pub fn parameterized(raw: TypeRef, args: Vec<TypeRef>) -> Self {
        TypeRef::Parameterized { raw: Box::new(raw), args }
    }

    /// Convenience: `TypeRef::wildcard_extends(TypeRef::simple("Number"))`
    pub fn wildcard_extends(bound: TypeRef) -> Self {
        TypeRef::Wildcard { bound: Some(WildcardBound::Extends(Box::new(bound))) }
    }

    /// Convenience: `TypeRef::wildcard_super(TypeRef::simple("Integer"))`
    pub fn wildcard_super(bound: TypeRef) -> Self {
        TypeRef::Wildcard { bound: Some(WildcardBound::Super(Box::new(bound))) }
    }

    /// Is this a primitive type?
    pub fn is_primitive(&self) -> bool {
        matches!(self, TypeRef::Primitive(_))
    }

    /// Is this `void`?
    pub fn is_void(&self) -> bool {
        matches!(self, TypeRef::Void)
    }

    /// Compute the erasure of this type (JLS §4.6).
    /// Type variables erase to their leftmost bound (or `Object`).
    /// Parameterized types erase to their raw type.
    /// Everything else erases to itself.
    pub fn erasure(&self) -> TypeRef {
        match self {
            TypeRef::Parameterized { raw, .. } => raw.erasure(),
            TypeRef::TypeVariable { .. } => TypeRef::simple("java.lang.Object"),
            TypeRef::Wildcard { bound } => match bound {
                Some(WildcardBound::Extends(t)) => t.erasure(),
                _ => TypeRef::simple("java.lang.Object"),
            },
            TypeRef::Array { element } => TypeRef::array(element.erasure()),
            TypeRef::Intersection { types } => {
                types.first().map(|t| t.erasure()).unwrap_or(TypeRef::simple("java.lang.Object"))
            }
            other => other.clone(),
        }
    }

    /// Substitute type variables using the given bindings.
    /// `substitute({"T": String, "U": Integer})` turns `Map<T, List<U>>` into `Map<String, List<Integer>>`.
    pub fn substitute(&self, bindings: &std::collections::HashMap<String, TypeRef>) -> TypeRef {
        match self {
            TypeRef::TypeVariable { name } => {
                bindings.get(name).cloned().unwrap_or_else(|| self.clone())
            }
            TypeRef::Parameterized { raw, args } => TypeRef::Parameterized {
                raw: Box::new(raw.substitute(bindings)),
                args: args.iter().map(|a| a.substitute(bindings)).collect(),
            },
            TypeRef::Array { element } => TypeRef::Array {
                element: Box::new(element.substitute(bindings)),
            },
            TypeRef::Wildcard { bound } => TypeRef::Wildcard {
                bound: bound.as_ref().map(|b| match b {
                    WildcardBound::Extends(t) => {
                        WildcardBound::Extends(Box::new(t.substitute(bindings)))
                    }
                    WildcardBound::Super(t) => {
                        WildcardBound::Super(Box::new(t.substitute(bindings)))
                    }
                }),
            },
            TypeRef::Intersection { types } => TypeRef::Intersection {
                types: types.iter().map(|t| t.substitute(bindings)).collect(),
            },
            other => other.clone(),
        }
    }
}

impl TypeParam {
    pub fn new(name: impl Into<String>) -> Self {
        TypeParam { name: name.into(), bounds: Vec::new() }
    }

    pub fn with_bounds(name: impl Into<String>, bounds: Vec<TypeRef>) -> Self {
        TypeParam { name: name.into(), bounds }
    }
}

// ---- Boxing ----

impl PrimitiveKind {
    /// Returns the FQN of the boxed wrapper type.
    pub fn boxed_fqn(&self) -> &'static str {
        match self {
            PrimitiveKind::Boolean => "java.lang.Boolean",
            PrimitiveKind::Byte => "java.lang.Byte",
            PrimitiveKind::Short => "java.lang.Short",
            PrimitiveKind::Int => "java.lang.Integer",
            PrimitiveKind::Long => "java.lang.Long",
            PrimitiveKind::Float => "java.lang.Float",
            PrimitiveKind::Double => "java.lang.Double",
            PrimitiveKind::Char => "java.lang.Character",
        }
    }

    /// Parse a primitive type name.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "boolean" => Some(PrimitiveKind::Boolean),
            "byte" => Some(PrimitiveKind::Byte),
            "short" => Some(PrimitiveKind::Short),
            "int" => Some(PrimitiveKind::Int),
            "long" => Some(PrimitiveKind::Long),
            "float" => Some(PrimitiveKind::Float),
            "double" => Some(PrimitiveKind::Double),
            "char" => Some(PrimitiveKind::Char),
            _ => None,
        }
    }

    /// Can this primitive be widened to `target`? (JLS §5.1.2)
    pub fn widens_to(&self, target: &PrimitiveKind) -> bool {
        use PrimitiveKind::*;
        matches!(
            (self, target),
            (Byte, Short | Int | Long | Float | Double)
                | (Short, Int | Long | Float | Double)
                | (Char, Int | Long | Float | Double)
                | (Int, Long | Float | Double)
                | (Long, Float | Double)
                | (Float, Double)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn display_simple_types() {
        assert_eq!(TypeRef::Void.to_string(), "void");
        assert_eq!(TypeRef::Primitive(PrimitiveKind::Int).to_string(), "int");
        assert_eq!(TypeRef::simple("String").to_string(), "String");
        assert_eq!(TypeRef::type_var("T").to_string(), "T");
        assert_eq!(TypeRef::Unknown.to_string(), "<unknown>");
    }

    #[test]
    fn display_parameterized() {
        let list_string = TypeRef::parameterized(
            TypeRef::simple("List"),
            vec![TypeRef::simple("String")],
        );
        assert_eq!(list_string.to_string(), "List<String>");

        let map = TypeRef::parameterized(
            TypeRef::simple("Map"),
            vec![TypeRef::simple("String"), TypeRef::simple("Integer")],
        );
        assert_eq!(map.to_string(), "Map<String, Integer>");
    }

    #[test]
    fn display_wildcards() {
        assert_eq!(
            TypeRef::Wildcard { bound: None }.to_string(),
            "?"
        );
        assert_eq!(
            TypeRef::wildcard_extends(TypeRef::simple("Number")).to_string(),
            "? extends Number"
        );
        assert_eq!(
            TypeRef::wildcard_super(TypeRef::simple("Integer")).to_string(),
            "? super Integer"
        );
    }

    #[test]
    fn display_array() {
        assert_eq!(
            TypeRef::array(TypeRef::Primitive(PrimitiveKind::Int)).to_string(),
            "int[]"
        );
        assert_eq!(
            TypeRef::array(TypeRef::array(TypeRef::simple("String"))).to_string(),
            "String[][]"
        );
    }

    #[test]
    fn erasure_parameterized() {
        let list_string = TypeRef::parameterized(
            TypeRef::simple("java.util.List"),
            vec![TypeRef::simple("java.lang.String")],
        );
        assert_eq!(list_string.erasure(), TypeRef::simple("java.util.List"));
    }

    #[test]
    fn erasure_type_variable() {
        let t = TypeRef::type_var("T");
        assert_eq!(t.erasure(), TypeRef::simple("java.lang.Object"));
    }

    #[test]
    fn erasure_wildcard() {
        let w = TypeRef::wildcard_extends(TypeRef::simple("Number"));
        assert_eq!(w.erasure(), TypeRef::simple("Number"));
    }

    #[test]
    fn substitute_type_variables() {
        // Map<T, List<U>>  with T=String, U=Integer
        let map_t_list_u = TypeRef::parameterized(
            TypeRef::simple("Map"),
            vec![
                TypeRef::type_var("T"),
                TypeRef::parameterized(
                    TypeRef::simple("List"),
                    vec![TypeRef::type_var("U")],
                ),
            ],
        );

        let mut bindings = HashMap::new();
        bindings.insert("T".to_string(), TypeRef::simple("String"));
        bindings.insert("U".to_string(), TypeRef::simple("Integer"));

        let result = map_t_list_u.substitute(&bindings);
        assert_eq!(result.to_string(), "Map<String, List<Integer>>");
    }

    #[test]
    fn primitive_widening() {
        assert!(PrimitiveKind::Int.widens_to(&PrimitiveKind::Long));
        assert!(PrimitiveKind::Int.widens_to(&PrimitiveKind::Double));
        assert!(!PrimitiveKind::Int.widens_to(&PrimitiveKind::Short));
        assert!(!PrimitiveKind::Long.widens_to(&PrimitiveKind::Int));
        assert!(PrimitiveKind::Char.widens_to(&PrimitiveKind::Int));
    }

    #[test]
    fn primitive_boxing() {
        assert_eq!(PrimitiveKind::Int.boxed_fqn(), "java.lang.Integer");
        assert_eq!(PrimitiveKind::Char.boxed_fqn(), "java.lang.Character");
    }
}
