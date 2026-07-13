use std::fmt;

/// Nesting is flat: `Foo$Inner` is its own class, linked back by `enclosing`.
#[derive(Debug, Clone)]
pub struct JvmClass {
    pub fqn: Fqn,
    pub kind: JvmKind,
    pub enclosing: Option<Fqn>,
    pub superclass: Option<Fqn>,
    pub interfaces: Vec<Fqn>,
    pub fields: Vec<JvmField>,
    pub methods: Vec<JvmMethod>,
}

/// Interfaces, enums, records and annotations are all class files with
/// different access flags; the JVM has no other top-level container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JvmKind {
    Class,
    Interface,
    Enum,
    Annotation,
    Record,
}

#[derive(Debug, Clone)]
pub struct JvmField {
    pub name: String,
    pub type_: JvmType,
}

#[derive(Debug, Clone)]
pub struct JvmMethod {
    pub name: String,
    pub params: Vec<JvmType>,
    pub return_type: JvmType,
}

/// Everything a JVM descriptor can encode, and nothing more.
/// Generics are erased: `List<String>` projects to `java.util.List`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JvmType {
    Primitive(JvmPrimitive),
    Class(Fqn),
    Array(Box<JvmType>),
    Void,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JvmPrimitive {
    Boolean,
    Byte,
    Char,
    Short,
    Int,
    Long,
    Float,
    Double,
}

/// Identity of a JVM type: the binary name, nested types joined with `$`.
/// e.g. `org.beans.app.Foo`, `org.beans.app.Foo$Inner`
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Fqn(String);

impl Fqn {
    pub fn new(binary_name: impl Into<String>) -> Fqn {
        Fqn(binary_name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn package(&self) -> &str {
        match self.0.rfind('.') {
            Some(dot) => &self.0[..dot],
            None => "",
        }
    }

    pub fn simple_name(&self) -> &str {
        match self.0.rfind(['.', '$']) {
            Some(sep) => &self.0[sep + 1..],
            None => &self.0,
        }
    }
}

impl fmt::Display for Fqn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_package_and_simple_name() {
        let fqn = Fqn::new("org.beans.app.Foo");
        assert_eq!(fqn.package(), "org.beans.app");
        assert_eq!(fqn.simple_name(), "Foo");
    }

    #[test]
    fn nested_type_is_named_by_its_last_segment() {
        let fqn = Fqn::new("org.beans.app.Foo$Inner");
        assert_eq!(fqn.package(), "org.beans.app");
        assert_eq!(fqn.simple_name(), "Inner");
    }

    #[test]
    fn default_package_has_no_qualifier() {
        let fqn = Fqn::new("Foo");
        assert_eq!(fqn.package(), "");
        assert_eq!(fqn.simple_name(), "Foo");
    }
}
