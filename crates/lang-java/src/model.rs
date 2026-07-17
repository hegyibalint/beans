use beans_core::Span;
use beans_platform_jvm::model::JvmQualifiedName;

#[derive(Debug, Default, Clone)]
pub struct JavaFile {
    pub package: Option<JavaQualifiedName>,
    pub imports: Vec<JavaImport>,
    pub classes: Vec<JavaClass>,
}

impl JavaFile {
    /// Every place a type is used (not declared): field, parameter and
    /// return types. Package and import names are not use sites.
    pub fn type_references(&self) -> impl Iterator<Item = &JavaQualifiedName> {
        self.classes.iter().flat_map(|class| {
            let field_types = class.fields.iter().map(|field| &field.java_type);
            let method_types = class.methods.iter().flat_map(|method| {
                std::iter::once(&method.return_type)
                    .chain(method.params.iter().map(|param| &param.java_type))
            });
            field_types.chain(method_types)
        })
    }
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
    pub java_type: JavaQualifiedName,
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
    pub java_type: JavaQualifiedName,
}

/// One or more simple names; "qualified" in the loose sense (a single
/// segment is legal, e.g. `package test;`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JavaQualifiedName {
    pub segments: Vec<JavaSimpleName>,
    pub span: Span,
}

impl JavaQualifiedName {
    pub fn dotted(&self) -> String {
        self.segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join(".")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JavaSimpleName {
    pub text: String,
    pub span: Span,
}

pub enum JavaSymbol {
    Resolved(JvmQualifiedName), // already bound: import / same-package / java.lang / same file
    Importable(Vec<JvmQualifiedName>), // not in scope, but the lake offers candidates
    Unresolvable,               // not found, and the searched scope was complete
    Unknown,                    // didn't do anything yet, default state
}
