// §6.6.1 asked of a declaration Beans never parsed. A class file gives an access
// level and a binary name and nothing else — no scope chain, no site — so the
// rule is decided from the package the name carries (§13.1) rather than from the
// declaration's surroundings, which is what `is_compiled_type_accessible` is.
//
// Until this arm existed every compiled type was accessible from everywhere, so
// each case below is one that used to resolve.

use super::*;

fn jar_class(
    jvm: &mut jvm::Platform,
    revision: Revision,
    fqn: &str,
    access: jvm::model::AccessLevel,
) -> jvm::model::Source {
    let source = jvm::model::Source::JarEntry {
        jar_path: PathBuf::from("lib.jar"),
        entry_path: format!("{}.class", fqn.replace('.', "/")),
    };
    compiled_class(jvm, revision, source, fqn, access)
}

fn resolve(
    java: &LanguageJava,
    jvm: &jvm::Platform,
    asker: &jvm::model::Source,
    name: &str,
) -> JavaTypeResolution {
    let revision = Revision::default();
    let file = file_model(java, revision, asker);
    let body = type_declaration(file, file.top_level_declarations[0]).body_scope;

    resolve_type_name(
        &JavaName::Simple(identifier(name)),
        asker,
        file,
        body,
        &java_query(java, jvm, revision),
    )
}

#[test]
fn a_package_private_compiled_type_is_reached_only_from_its_own_package() {
    let revision = Revision::default();
    let mut java = LanguageJava::new();
    let mut jvm = jvm::Platform::new();
    let jar = jar_class(&mut jvm, revision, "p.X", jvm::model::AccessLevel::Package);
    let neighbour = process(
        &mut java,
        &mut jvm,
        revision,
        "p/Test.java",
        "package p; class Test {}",
    );
    let stranger = process(
        &mut java,
        &mut jvm,
        revision,
        "q/Test.java",
        "package q; import p.X; class Test {}",
    );

    assert_eq!(
        resolve(&java, &jvm, &neighbour, "X"),
        JavaTypeResolution::Resolved(JavaTypeTarget::Jvm {
            source: jar,
            fqn: jvm::model::BinaryName::new("p.X"),
        })
    );

    let JavaTypeResolution::Unresolved { invalid_candidates } =
        resolve(&java, &jvm, &stranger, "X")
    else {
        panic!("another package must not reach a package-private class");
    };
    assert_eq!(invalid_candidates.len(), 1);
    assert!(invalid_candidates[0].has_invalidity(JavaTypeInvalidity::Inaccessible));
}

/// A member of a compiled type, which is where the binary name stops being the
/// canonical one: §13.1 joins it with `$`, and §6.5.4.2 walks the import that
/// spells it with a dot. §6.6.1 then answers `private` the only way it can from
/// outside a class file — no Java source is ever inside that body.
#[test]
fn a_private_member_of_a_compiled_type_is_reached_by_nobody() {
    let revision = Revision::default();
    let mut java = LanguageJava::new();
    let mut jvm = jvm::Platform::new();
    jar_class(
        &mut jvm,
        revision,
        "p.Outer",
        jvm::model::AccessLevel::Public,
    );
    jar_class(
        &mut jvm,
        revision,
        "p.Outer$Inner",
        jvm::model::AccessLevel::Private,
    );
    let asker = process(
        &mut java,
        &mut jvm,
        revision,
        "p/Test.java",
        "package p; import p.Outer.Inner; class Test {}",
    );

    let JavaTypeResolution::Unresolved { invalid_candidates } =
        resolve(&java, &jvm, &asker, "Inner")
    else {
        panic!("a private member of a class file must not resolve");
    };
    assert_eq!(invalid_candidates.len(), 1);
    assert!(invalid_candidates[0].has_invalidity(JavaTypeInvalidity::Inaccessible));
}

/// The other half of the walk above: the same spelling, the same `$` join, and a
/// member that is public reaches its declaration.
#[test]
fn a_public_member_of_a_compiled_type_is_reached_through_its_dollar_name() {
    let revision = Revision::default();
    let mut java = LanguageJava::new();
    let mut jvm = jvm::Platform::new();
    jar_class(
        &mut jvm,
        revision,
        "p.Outer",
        jvm::model::AccessLevel::Public,
    );
    let inner = jar_class(
        &mut jvm,
        revision,
        "p.Outer$Inner",
        jvm::model::AccessLevel::Public,
    );
    let asker = process(
        &mut java,
        &mut jvm,
        revision,
        "q/Test.java",
        "package q; import p.Outer.Inner; class Test {}",
    );

    assert_eq!(
        resolve(&java, &jvm, &asker, "Inner"),
        JavaTypeResolution::Resolved(JavaTypeTarget::Jvm {
            source: inner,
            fqn: jvm::model::BinaryName::new("p.Outer$Inner"),
        })
    );
}
