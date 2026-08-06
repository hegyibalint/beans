use beans_platform_jvm as jvm;

use crate::model;

/// Declarations only for now: the class identities fall out of the file
/// alone, while members and supertypes need resolution against the lake.
pub fn project_to_jvm(file: &model::File) -> Vec<jvm::model::Class> {
    let package = file.package.as_ref().map(|name| name.dotted());

    file.top_level_declarations
        .iter()
        .filter_map(|id| {
            let model::Declaration::Type(declaration) = file.declarations.get(id.0)? else {
                return None;
            };
            let name = declaration.name.as_ref()?.text.clone();
            let binary_name = match &package {
                Some(package) => format!("{package}.{name}"),
                None => name,
            };
            let kind = match declaration.kind {
                model::TypeKind::Class => jvm::model::TypeKind::Class,
                model::TypeKind::Interface => jvm::model::TypeKind::Interface,
                model::TypeKind::Enum => jvm::model::TypeKind::Enum,
                model::TypeKind::Record => jvm::model::TypeKind::Record,
                model::TypeKind::AnnotationInterface => jvm::model::TypeKind::AnnotationInterface,
            };
            Some(jvm::model::Class {
                fqn: jvm::model::BinaryName::new(binary_name),
                kind,
                access: declaration.access.map(|access| match access.level {
                    model::AccessLevel::Public => jvm::model::AccessLevel::Public,
                    model::AccessLevel::Protected => jvm::model::AccessLevel::Protected,
                    model::AccessLevel::Package => jvm::model::AccessLevel::Package,
                    model::AccessLevel::Private => jvm::model::AccessLevel::Private,
                }),
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
    use crate::parser::Parser;

    #[test]
    fn classes_project_to_package_qualified_fqns() {
        let mut parser = Parser::new();
        let model = parser.parse("package org.beans.app;\n\nclass Foo {}\nclass Helper {}\n");

        let classes = project_to_jvm(&model);
        let fqns: Vec<&str> = classes.iter().map(|c| c.fqn.as_str()).collect();
        assert_eq!(fqns, ["org.beans.app.Foo", "org.beans.app.Helper"]);
    }

    /// What the lake is told about access when the declaration is one we parsed,
    /// which is the other half of what `class_file.rs` decodes for a compiled
    /// one. §7.6 gives a top level type `public` or package access and nothing
    /// else, and projection walks top level declarations only, so those are the
    /// two a legal program can reach here; the other arms wait for nested types
    /// to be projected at all.
    #[test]
    fn a_top_level_type_projects_the_access_level_it_was_declared_with() {
        let mut parser = Parser::new();
        let model = parser.parse("package p;\n\npublic class Open {}\nclass Closed {}\n");

        let classes = project_to_jvm(&model);
        let projected: Vec<(&str, Option<jvm::model::AccessLevel>)> = classes
            .iter()
            .map(|class| (class.fqn.as_str(), class.access))
            .collect();

        assert_eq!(
            projected,
            [
                ("p.Open", Some(jvm::model::AccessLevel::Public)),
                ("p.Closed", Some(jvm::model::AccessLevel::Package)),
            ]
        );
    }

    #[test]
    fn default_package_projects_bare_names() {
        let mut parser = Parser::new();
        let model = parser.parse("class Foo {}\n");

        let classes = project_to_jvm(&model);
        assert_eq!(classes[0].fqn.as_str(), "Foo");
    }
}
