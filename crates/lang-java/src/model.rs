use beans_core::Span;
use beans_platform_jvm::model::Fqn;

#[derive(Debug, Default, Clone)]
pub struct JavaFile {
    pub package: Option<JavaQualifiedName>,
    pub imports: Vec<JavaImport>,
    pub classes: Vec<JavaClass>,
}

#[derive(Debug, Clone)]
pub struct JavaImport {
    pub name: JavaQualifiedName,
    pub kind: JavaImportKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaImportKind {
    Type,           // import a.b.C;
    TypeOnDemand,   // import a.b.*;
    Static,         // import static a.b.C.member;
    StaticOnDemand, // import static a.b.C.*;
    Module,         // import module a.b;  (Java 25)
}

#[derive(Debug, Clone)]
pub struct JavaClass {
    pub name: JavaSimpleName,
    pub fields: Vec<JavaField>,
    pub methods: Vec<JavaMethod>,
}

#[derive(Debug, Clone)]
pub struct JavaField {
    pub name: JavaSimpleName,
    pub type_: JavaQualifiedName,
}

#[derive(Debug, Clone)]
pub struct JavaMethod {
    pub name: JavaSimpleName,
    pub params: Vec<JavaMethodParameter>,
    pub return_type: JavaQualifiedName,
}

#[derive(Debug, Clone)]
pub struct JavaMethodParameter {
    pub name: JavaSimpleName,
    pub type_: JavaQualifiedName,
}

/// One or more simple names; "qualified" in the loose sense (a single
/// segment is legal, e.g. `package test;`).
#[derive(Debug, Clone)]
pub struct JavaQualifiedName {
    pub segments: Vec<JavaSimpleName>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct JavaSimpleName {
    pub text: String,
    pub span: Span,
}

pub enum JavaSymbol {
    Resolved(Fqn),        // already bound: import / same-package / java.lang / same file
    Importable(Vec<Fqn>), // not in scope, but the catalog offers candidates
    Unresolved,           // catalog has nothing
}
