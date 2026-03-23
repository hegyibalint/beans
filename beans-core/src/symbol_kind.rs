#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    // Shared JVM
    Class,
    Interface,
    Enum,
    Record,
    Annotation,
    Method,
    Constructor,
    Field,
    Parameter,
    Package,

    // Kotlin-specific
    Object,
    CompanionObject,
    DataClass,
    SealedClass,

    // Scala-specific
    Trait,
    CaseClass,
    CaseObject,

    // Clojure-specific
    Namespace,
    Function,
    Protocol,
    Multimethod,
    Defrecord,
    Deftype,
}
