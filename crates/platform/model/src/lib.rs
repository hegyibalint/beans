use std::fmt;

/// A class or interface binary name in the external form defined by JLS §13.1.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BinaryName(String);

impl BinaryName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BinaryName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassKind {
    Class,
    Enum,
    Record,
    Interface,
    AnnotationInterface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessLevel {
    Public,
    Protected,
    Package,
    Private,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberClass {
    pub simple_name: String,
    pub binary_name: BinaryName,
}

impl MemberClass {
    pub fn new(simple_name: impl Into<String>, binary_name: BinaryName) -> Self {
        Self {
            simple_name: simple_name.into(),
            binary_name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class {
    pub binary_name: BinaryName,
    pub kind: ClassKind,
    pub access: AccessLevel,
    pub member_classes: Vec<MemberClass>,
}

impl Class {
    pub fn new(binary_name: BinaryName, kind: ClassKind, access: AccessLevel) -> Self {
        Self {
            binary_name,
            kind,
            access,
            member_classes: Vec::new(),
        }
    }

    pub fn member_classes_named<'a>(
        &'a self,
        name: &'a str,
    ) -> impl Iterator<Item = &'a MemberClass> + 'a {
        self.member_classes
            .iter()
            .filter(move |member| member.simple_name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_names_use_the_jls_external_form() {
        for name in ["Example", "java.lang.String", "example.Outer$Member"] {
            let binary_name = BinaryName::new(name);

            assert_eq!(binary_name.as_str(), name);
            assert_eq!(binary_name.to_string(), name);
        }
    }

    #[test]
    fn member_lookup_uses_source_names_and_preserves_duplicate_declarations() {
        let mut class = Class::new(
            BinaryName::new("example.Outer"),
            ClassKind::Class,
            AccessLevel::Public,
        );
        class.member_classes = vec![
            MemberClass::new("Other", BinaryName::new("example.Outer$Other")),
            MemberClass::new("Member", BinaryName::new("example.Outer$Member")),
            MemberClass::new("Member", BinaryName::new("example.Outer$Duplicate")),
        ];

        let found: Vec<_> = class
            .member_classes_named("Member")
            .map(|member| member.binary_name.as_str())
            .collect();

        assert_eq!(found, ["example.Outer$Member", "example.Outer$Duplicate"]);
    }
}
