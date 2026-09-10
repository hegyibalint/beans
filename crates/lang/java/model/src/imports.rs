use crate::{names::Name, references::TypeRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    name: Name,
    typ: ImportType,
}

impl Import {
    pub fn new(name: Name, typ: ImportType) -> Self {
        Self { name, typ }
    }

    pub fn name(&self) -> &Name {
        &self.name
    }

    pub fn typ(&self) -> ImportType {
        self.typ
    }

    /// Checks whether a single import's simple name starts this reference (JLS §7.5.1, §7.5.3).
    /// Matches spelling only, not import validity or whether the target is a type.
    pub fn is_prefix(&self, type_ref: &TypeRef) -> bool {
        if self.typ != ImportType::SingleType && self.typ != ImportType::SingleStaticType {
            return false;
        }
        let Some(imported_name) = self.name.as_slice().last() else {
            return false;
        };
        let TypeRef::Named { segments } = type_ref else {
            return false;
        };
        let Some(first) = segments.first() else {
            return false;
        };

        !imported_name.is_empty() && imported_name == &first.name
    }

    /// Checks required, nonempty name components (JLS §7.5.1–§7.5.5), not target validity.
    pub fn is_name_valid(&self) -> bool {
        let minimum_components = match self.typ {
            ImportType::SingleType | ImportType::OnDemandStaticType => 2,
            ImportType::SingleStaticType => 3,
            ImportType::OnDemandType | ImportType::SingleModule => 1,
        };

        self.name.len() >= minimum_components && self.name.is_valid()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportType {
    /// JLS 7.5.1 Single-Type-Import Declarations
    ///
    /// `import java.util.List;`
    SingleType,
    /// JLS 7.5.2 Type-Import-on-Demand Declarations
    ///
    /// `import java.util.*;`
    OnDemandType,
    /// JLS 7.5.3 Single-Static-Import Declarations
    ///
    /// `import static java.util.Objects.requireNonNull;`
    SingleStaticType,
    /// JLS 7.5.4 Static-Import-on-Demand Declarations
    ///
    /// `import static java.util.Comparator.*;`
    OnDemandStaticType,
    /// JLS 7.5.5 Single-Module-Import Declarations
    ///
    /// `import module java.base;`
    SingleModule,
}

#[cfg(test)]
mod tests {
    use super::{Import, ImportType::*};
    use crate::{
        names::Name,
        references::{PrimitiveType, TypeBound, TypeNameComponent, TypeRef},
    };

    fn named(value: &str) -> TypeRef {
        TypeRef::Named {
            segments: value
                .split('.')
                .map(|name| TypeNameComponent {
                    name: name.into(),
                    bounds: vec![],
                })
                .collect(),
        }
    }

    #[test]
    fn single_imports_match_the_starting_simple_name_with_or_without_members() {
        for typ in [SingleType, SingleStaticType] {
            let import = Import::new(
                Name::new(vec!["p".into(), "Container".into(), "Outer".into()]),
                typ,
            );
            for reference in ["Outer", "Outer.Inner", "Outer.Inner.Deep"] {
                assert!(import.is_prefix(&named(reference)), "{typ:?}: {reference}");
            }
            for reference in [
                "Inner",
                "Container.Outer",
                "p.Container.Outer",
                "Other.Outer",
                "OuterExtra",
                "outer",
            ] {
                assert!(!import.is_prefix(&named(reference)), "{typ:?}: {reference}");
            }
        }
    }

    #[test]
    fn on_demand_and_module_imports_do_not_bind_their_last_component() {
        for typ in [OnDemandType, OnDemandStaticType, SingleModule] {
            let import = Import::new(Name::new(vec!["p".into(), "Outer".into()]), typ);
            assert!(!import.is_prefix(&named("Outer.Inner")));
        }
    }

    #[test]
    fn missing_names_and_non_named_references_have_no_import_prefix() {
        let import = Import::new(Name::new(vec!["p".into(), "Outer".into()]), SingleType);
        for reference in [
            TypeRef::Named { segments: vec![] },
            named(""),
            TypeRef::Primitive(PrimitiveType::Int),
            TypeRef::Void,
            TypeRef::Array {
                element: Box::new(named("Outer")),
                dimensions: 1,
            },
        ] {
            assert!(!import.is_prefix(&reference));
        }
        for name in [vec![], vec!["".into()]] {
            let import = Import::new(Name::new(name), SingleType);
            assert!(!import.is_prefix(&named("Outer")));
            assert!(!import.is_prefix(&named("")));
        }
    }

    #[test]
    fn type_arguments_do_not_change_the_starting_name() {
        let import = Import::new(Name::new(vec!["p".into(), "Outer".into()]), SingleType);
        let mut reference = named("Outer.Inner");
        let TypeRef::Named { segments } = &mut reference else {
            unreachable!();
        };
        segments[0].bounds.push(TypeBound::Unbounded);

        assert!(import.is_prefix(&reference));
    }

    #[test]
    fn matching_a_prefix_does_not_validate_the_import() {
        let import = Import::new(Name::new(vec!["Outer".into()]), SingleType);

        assert!(import.is_prefix(&named("Outer.Inner")));
        assert!(!import.is_name_valid());
    }

    #[test]
    fn each_import_kind_accepts_minimum_and_longer_name_shapes() {
        for (typ, name) in [
            (SingleType, "p.Type"),
            (SingleStaticType, "p.Type.member"),
            (OnDemandStaticType, "p.Type"),
            (OnDemandType, "p"),
            (SingleModule, "m"),
            (SingleType, "java.util.List"),
            (SingleStaticType, "java.util.Collections.emptyList"),
            (OnDemandStaticType, "java.util.Collections"),
            (OnDemandType, "java.lang"),
            (SingleModule, "java.base"),
        ] {
            let import = Import::new(name.split('.').map(str::to_owned).collect(), typ);
            assert!(import.is_name_valid(), "{typ:?}: {name}");
        }
    }

    #[test]
    fn type_imports_require_room_for_a_named_package() {
        for (typ, name) in [
            (SingleType, "Type"),
            (SingleStaticType, "member"),
            (SingleStaticType, "Type.member"),
            (OnDemandStaticType, "Type"),
        ] {
            let import = Import::new(name.split('.').map(str::to_owned).collect(), typ);
            assert!(!import.is_name_valid(), "{typ:?}: {name}");
        }
    }

    #[test]
    fn every_import_kind_rejects_a_missing_name() {
        for typ in [
            SingleType,
            SingleStaticType,
            OnDemandStaticType,
            OnDemandType,
            SingleModule,
        ] {
            assert!(!Import::new(Name::default(), typ).is_name_valid());
        }
    }

    #[test]
    fn invalid_components_make_an_import_name_invalid_despite_sufficient_length() {
        let name = Name::new(vec!["p".into(), "".into(), "Member".into()]);
        let import = Import::new(name, SingleType);

        assert!(!import.is_name_valid());
    }
}
