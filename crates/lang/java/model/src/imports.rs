#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    name: Vec<String>,
    typ: ImportType,
}

impl Import {
    pub fn new(name: Vec<String>, typ: ImportType) -> Self {
        Self { name, typ }
    }

    pub fn name(&self) -> &[String] {
        &self.name
    }

    pub fn typ(&self) -> ImportType {
        self.typ
    }

    /// Checks required, nonempty name components (JLS §7.5.1–§7.5.5), not target validity.
    pub fn has_valid_name_shape(&self) -> bool {
        let minimum_components = match self.typ {
            ImportType::SingleType | ImportType::OnDemandStaticType => 2,
            ImportType::SingleStaticType => 3,
            ImportType::OnDemandType | ImportType::SingleModule => 1,
        };

        self.name.len() >= minimum_components && self.name.iter().all(|part| !part.is_empty())
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
            assert!(import.has_valid_name_shape(), "{typ:?}: {name}");
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
            assert!(!import.has_valid_name_shape(), "{typ:?}: {name}");
        }
    }

    #[test]
    fn every_import_kind_rejects_missing_or_empty_name_components() {
        for typ in [
            SingleType,
            SingleStaticType,
            OnDemandStaticType,
            OnDemandType,
            SingleModule,
        ] {
            assert!(!Import::new(vec![], typ).has_valid_name_shape());
            for name in [".Outer.member", "p..member", "p.Outer."] {
                let import = Import::new(name.split('.').map(str::to_owned).collect(), typ);
                assert!(!import.has_valid_name_shape(), "{typ:?}: {name}");
            }
        }
    }
}
