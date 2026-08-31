use crate::NameRef;

pub struct Import {
    name: NameRef,
    typ: ImportType,
}

pub enum ImportType {
    /// JLS 7.5.1 Single-Type-Import Declarations
    SingleType,
    /// JLS 7.5.2 Type-Import-on-Demand Declarations
    OnDemandType,
    /// JLS 7.5.3 Single-Static-Import Declarations
    SingleStaticType,
    /// JLS 7.5.4 Static-Import-on-Demand Declarations
    OnDemandStaticType,
    /// JLS 7.5.5 Single-Module-Import Declarations
    SingleModule,
}
