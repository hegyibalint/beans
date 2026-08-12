use std::{fmt, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Source {
    SourceFile {
        /// The filesystem path to the source file, e.g. `src/main/java/org/beans/app/Foo.java`.
        path: PathBuf,
    },
    ClassFile {
        /// The filesystem path to a standalone class file, e.g. `build/classes/org/beans/app/Foo.class`.
        path: PathBuf,
    },
    JarEntry {
        /// The filesystem path to the jar file, e.g. `.m2/repository/org/beans/app/1.0.0/app-1.0.0.jar`.
        jar_path: PathBuf,
        /// The logical path to the entry within the jar file, e.g. `org/beans/app/Foo.class`.
        entry_path: String,
    },
    JmodEntry {
        /// The filesystem path to the jmod file, e.g. `/usr/lib/jvm/java-17-openjdk-amd64/jmods/java.base.jmod`.
        jmod_path: PathBuf,
        /// The logical path to the entry within the jmod file, e.g. `classes/dev/blnt/beans/app/Foo.class`.
        entry_path: String,
    },
    JimageEntry {
        /// The filesystem path to the runtime image, e.g. `/usr/lib/jvm/java-17-openjdk-amd64/lib/modules`.
        /// A JDK has exactly one, holding every system module.
        jimage_path: PathBuf,
        /// The logical path to the entry within the image, e.g. `java.base/java/lang/String.class`.
        entry_path: String,
    },
}

/// Nesting is flat: `Foo$Inner` is its own class, linked back by `enclosing`.
#[derive(Debug, Clone)]
pub struct Class {
    pub fqn: BinaryName,
    pub kind: TypeKind,
    /// `None` where JLS §8.1.1 says access control does not apply: a local or
    /// anonymous class.
    pub access: Option<AccessLevel>,
    pub enclosing: Option<BinaryName>,
    pub superclass: Option<BinaryName>,
    pub interfaces: Vec<BinaryName>,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
}

/// The one thing JLS §6.6.1 asks of a declaration, decoded from wherever the
/// class file happens to keep it: JVMS §4.1 for a top-level class, §4.7.6 for a
/// nested one, §4.5 and §4.6 for a field and a method. None of the three bits
/// set means package access, which is why this is four values and not three
/// booleans a caller has to combine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessLevel {
    Public,
    Protected,
    Package,
    Private,
}

/// Projection fills this from a source declaration, so a record costs nothing
/// here. A reader of real class files gets the other four from `access_flags`
/// (JVMS §4.1) and has to go find the `Record` attribute for this one (§4.7.30).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    Class,
    Interface,
    Enum,
    Record,
    AnnotationInterface,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub access: AccessLevel,
    pub jvm_type: Type,
}

#[derive(Debug, Clone)]
pub struct Method {
    pub name: String,
    pub access: AccessLevel,
    pub params: Vec<Type>,
    pub return_type: ReturnType,
}

/// A field, parameter or array component type: JVMS §4.3.2's `FieldType`.
/// Generics are erased: `List<String>` projects to `java.util.List`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Primitive(Primitive),
    Class(BinaryName),
    Array(Box<Type>),
}

/// What a method hands back: JVMS §4.3.3's `ReturnDescriptor`. The one position
/// where `V` is legal, which is why void is not a `Type`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ReturnType {
    Value(Type),
    Void,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Primitive {
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
pub struct BinaryName(String);

impl BinaryName {
    pub fn new(binary_name: impl Into<String>) -> BinaryName {
        BinaryName(binary_name.into())
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

    /// A top-level type in `package`, whose binary name is its canonical name
    /// (JLS 26 §13.1). An empty package is the unnamed one, with no dot to add.
    pub fn in_package(package: &str, simple_name: &str) -> BinaryName {
        if package.is_empty() {
            BinaryName(simple_name.to_string())
        } else {
            BinaryName(format!("{package}.{simple_name}"))
        }
    }

    /// A member type of this one: the enclosing binary name, `$`, the simple
    /// name (JLS 26 §13.1). Local and anonymous classes take a digit sequence
    /// after the `$` and cannot be spelled this way.
    pub fn nested(&self, simple_name: &str) -> BinaryName {
        BinaryName(format!("{}${simple_name}", self.0))
    }

    /// Whether this names a type declared inside another, which §13.1 spells
    /// with a `$`. A top-level class whose own identifier contains one is legal
    /// (§3.8) and reads the same way; javac calls the collision a duplicate
    /// class rather than distinguishing them, so neither can we.
    pub fn is_nested(&self) -> bool {
        self.0.contains('$')
    }
}

impl fmt::Display for BinaryName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_package_and_simple_name() {
        let fqn = BinaryName::new("org.beans.app.Foo");
        assert_eq!(fqn.package(), "org.beans.app");
        assert_eq!(fqn.simple_name(), "Foo");
    }

    #[test]
    fn nested_type_is_named_by_its_last_segment() {
        let fqn = BinaryName::new("org.beans.app.Foo$Inner");
        assert_eq!(fqn.package(), "org.beans.app");
        assert_eq!(fqn.simple_name(), "Inner");
    }

    #[test]
    fn default_package_has_no_qualifier() {
        let fqn = BinaryName::new("Foo");
        assert_eq!(fqn.package(), "");
        assert_eq!(fqn.simple_name(), "Foo");
    }
}
