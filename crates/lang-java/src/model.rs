use beans_core::model::Span;

#[derive(Debug)]
pub struct JavaSimpleName {
    pub text: String,
    pub span: Span,
}

/// One or more simple names; "qualified" in the loose sense (a single
/// segment is legal, e.g. `package test;`).
#[derive(Debug)]
pub struct JavaQualifiedName {
    pub segments: Vec<JavaSimpleName>,
    pub span: Span,
}

#[derive(Debug)]
pub enum JavaImportKind {
    Type,            // import a.b.C;
    TypeOnDemand,    // import a.b.*;
    Static,          // import static a.b.C.member;
    StaticOnDemand,  // import static a.b.C.*;
    Module,          // import module a.b;  (Java 25)
}

#[derive(Debug)]
pub struct JavaImport {
    pub name: JavaQualifiedName,
    pub kind: JavaImportKind,
}

#[derive(Debug)]
pub struct JavaClass {
    pub name: JavaSimpleName,
}

#[derive(Debug, Default)]
pub struct JavaFile {
    pub package: Option<JavaQualifiedName>,
    pub imports: Vec<JavaImport>,
    pub classes: Vec<JavaClass>,
}
