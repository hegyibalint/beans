//! Stage 4, §7.3: every compilation unit is treated as if `import java.lang.*;`
//! stood after its package declaration.

use super::*;

#[test]
fn a_java_lang_type_is_offered_without_an_import() {
    let items = Workspace::of(&[(
        "p/Test.java",
        "package p;
class Test {
    <cur> field;
}
",
    )])
    .compiled("jdk/java/lang/String.class", "java.lang.String")
    .complete();

    assert!(labels(&items).contains(&"String"));
    assert_eq!(
        item(&items, "String").detail.as_deref(),
        Some("java.lang.String")
    );
}

/// §7.3 imports the `public` types of `java.lang`, and a runtime image is full
/// of the others. The stage runs every name through `classify_type_target`, so
/// §6.6.1 keeps the package-private ones out without this stage knowing why.
#[test]
fn a_package_private_java_lang_type_is_not_offered() {
    let mut workspace = Workspace::of(&[(
        "p/Test.java",
        "package p;
class Test {
    <cur> field;
}
",
    )]);
    let at = workspace.revision.bump();
    workspace.jvm.register(
        at,
        jvm::model::Source::ClassFile {
            path: std::path::PathBuf::from("jdk/java/lang/Shutdown.class"),
        },
        vec![jvm::model::Class {
            fqn: jvm::model::BinaryName::new("java.lang.Shutdown"),
            kind: jvm::model::TypeKind::Class,
            access: Some(jvm::model::AccessLevel::Package),
            enclosing: None,
            superclass: None,
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
        }],
    );

    let items = workspace.complete();

    assert!(!labels(&items).contains(&"Shutdown"));
}
