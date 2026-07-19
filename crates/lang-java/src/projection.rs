use beans_platform_jvm::model::{JvmClass, JvmKind, JvmQualifiedName};

use crate::model::{JavaDeclaration, JavaFile, JavaTypeKind};

/// Declarations only for now: the class identities fall out of the file
/// alone, while members and supertypes need resolution against the lake.
pub fn project_to_jvm(file: &JavaFile) -> Vec<JvmClass> {
    let package = file.package.as_ref().map(|name| name.dotted());

    file.top_level_types
        .iter()
        .filter_map(|id| {
            let JavaDeclaration::Type(declaration) = file.declarations.get(id.0)? else {
                return None;
            };
            let name = declaration.name.as_ref()?.text.clone();
            let binary_name = match &package {
                Some(package) => format!("{package}.{name}"),
                None => name,
            };
            let kind = match declaration.kind {
                JavaTypeKind::Class => JvmKind::Class,
                JavaTypeKind::Interface => JvmKind::Interface,
                JavaTypeKind::Enum => JvmKind::Enum,
                JavaTypeKind::Record => JvmKind::Record,
                JavaTypeKind::AnnotationInterface => JvmKind::Annotation,
            };
            Some(JvmClass {
                fqn: JvmQualifiedName::new(binary_name),
                kind,
                enclosing: None,
                superclass: None,
                interfaces: Vec::new(),
                fields: Vec::new(),
                methods: Vec::new(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::JavaParser;

    #[test]
    fn classes_project_to_package_qualified_fqns() {
        let mut parser = JavaParser::new();
        let model = parser.parse("package org.beans.app;\n\nclass Foo {}\nclass Helper {}\n");

        let classes = project_to_jvm(&model);
        let fqns: Vec<&str> = classes.iter().map(|c| c.fqn.as_str()).collect();
        assert_eq!(fqns, ["org.beans.app.Foo", "org.beans.app.Helper"]);
    }

    #[test]
    fn default_package_projects_bare_names() {
        let mut parser = JavaParser::new();
        let model = parser.parse("class Foo {}\n");

        let classes = project_to_jvm(&model);
        assert_eq!(classes[0].fqn.as_str(), "Foo");
    }
}
