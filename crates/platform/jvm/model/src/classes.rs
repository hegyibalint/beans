use crate::names::BinaryName;

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
    use super::{AccessLevel, Class, ClassKind, MemberClass};
    use crate::names::BinaryName;

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
