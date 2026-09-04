use crate::references::NameRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    name: NameRef,
    typ: ImportType,
}

impl Import {
    pub fn new(name: NameRef, typ: ImportType) -> Self {
        Self { name, typ }
    }

    pub fn name(&self) -> &NameRef {
        &self.name
    }

    pub fn typ(&self) -> ImportType {
        self.typ
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
