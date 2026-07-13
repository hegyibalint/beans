use beans_platform_jvm::model::{Fqn, JvmClass, JvmKind};

use crate::model::JavaFile;

/// Declarations only for now: the class identities fall out of the file
/// alone, while members and supertypes need resolution against the lake.
pub fn project_to_jvm(file: &JavaFile) -> Vec<JvmClass> {
    let package = file.package.as_ref().map(|name| {
        name.segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join(".")
    });

    file.classes
        .iter()
        .map(|class| {
            let binary_name = match &package {
                Some(package) => format!("{package}.{}", class.name.text),
                None => class.name.text.clone(),
            };
            JvmClass {
                fqn: Fqn::new(binary_name),
                kind: JvmKind::Class,
                enclosing: None,
                superclass: None,
                interfaces: Vec::new(),
                fields: Vec::new(),
                methods: Vec::new(),
            }
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
