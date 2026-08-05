// §7.3 has every compilation unit behave as if `import java.lang.*;` stood after
// its package declaration, and stage 4 of `resolve_type_name` is that import: it
// glues the one package onto a simple name and asks the lake. The on-demand
// imports a user writes (§7.5.2) are not read yet.
//
// `java.lang` is a runtime image here rather than a source file, because that is
// where it comes from once a project names a JDK, and it is the arm of
// resolution that has no Java model behind it. Which stage wins when several can
// answer is settled in the `staging.rs`.

use super::*;

fn runtime_class(
    jvm: &mut PlatformJvm,
    revision: Revision,
    fqn: &str,
    access: JvmAccessLevel,
) -> JvmSource {
    let source = JvmSource::JimageEntry {
        jimage_path: PathBuf::from("jdk/lib/modules"),
        entry_path: format!("java.base/{}.class", fqn.replace('.', "/")),
    };
    compiled_class(jvm, revision, source, fqn, access)
}

/// The name as a reference in the asking file's class body reaches it.
fn resolve(
    java: &LanguageJava,
    query: &JavaQuery,
    asker: &JvmSource,
    name: &str,
) -> JavaTypeResolution {
    let file = file_model(java, Revision::default(), asker);
    let body = type_declaration(file, file.top_level_declarations[0]).body_scope;

    resolve_type_name(
        &JavaName::Simple(identifier(name)),
        asker,
        file,
        body,
        query,
    )
}

#[test]
fn a_simple_name_reaches_java_lang_without_an_import() {
    let revision = Revision::default();
    let mut java = LanguageJava::new();
    let mut jvm = PlatformJvm::new();
    let runtime = runtime_class(
        &mut jvm,
        revision,
        "java.lang.String",
        JvmAccessLevel::Public,
    );
    let asker = process(
        &mut java,
        &mut jvm,
        revision,
        "p/Test.java",
        "package p; class Test {}",
    );

    assert_eq!(
        resolve(&java, &java_query(&java, &jvm, revision), &asker, "String"),
        JavaTypeResolution::Resolved(JavaTypeTarget::Jvm {
            source: runtime,
            fqn: JvmQualifiedName::new("java.lang.String"),
        })
    );
}

/// §7.3 names one package. The rest of the runtime is reachable, and only by
/// being named: a JDK in scope must not put `List` in scope with it.
#[test]
fn no_other_package_of_the_runtime_is_implicitly_imported() {
    let revision = Revision::default();
    let mut java = LanguageJava::new();
    let mut jvm = PlatformJvm::new();
    runtime_class(&mut jvm, revision, "java.util.List", JvmAccessLevel::Public);
    let asker = process(
        &mut java,
        &mut jvm,
        revision,
        "p/Test.java",
        "package p; class Test {}",
    );

    assert_eq!(
        resolve(&java, &java_query(&java, &jvm, revision), &asker, "List"),
        JavaTypeResolution::Unresolved {
            invalid_candidates: Vec::new(),
        }
    );
}

/// §7.3 imports every `public` class of `java.lang` and no other, and the
/// runtime is full of the others: `java.lang.Shutdown` is package-private, so a
/// bare `Shutdown` reaches nothing.
///
/// The two do not agree on why. §7.3 never imports the name, so it is not in
/// scope at all; Beans finds the class and refuses it under §6.6.1, which leaves
/// the candidate behind as evidence. Both are unresolved, and the evidence is
/// what lets a diagnostic say something better than "cannot find symbol".
#[test]
fn a_package_private_class_of_java_lang_is_not_imported() {
    let revision = Revision::default();
    let mut java = LanguageJava::new();
    let mut jvm = PlatformJvm::new();
    runtime_class(
        &mut jvm,
        revision,
        "java.lang.Shutdown",
        JvmAccessLevel::Package,
    );
    let asker = process(
        &mut java,
        &mut jvm,
        revision,
        "p/Test.java",
        "package p; class Test {}",
    );

    let JavaTypeResolution::Unresolved { invalid_candidates } = resolve(
        &java,
        &java_query(&java, &jvm, revision),
        &asker,
        "Shutdown",
    ) else {
        panic!("a package-private class of java.lang must not resolve");
    };
    assert_eq!(invalid_candidates.len(), 1);
    assert!(invalid_candidates[0].has_invalidity(JavaTypeInvalidity::Inaccessible));
}

/// The implicit import is not a promise that `java.lang` is there. A unit naming
/// no JDK reaches the image nobody gave it as evidence and nothing else, which is
/// what puts a `type-outside-scope` on a bare `String`.
#[test]
fn a_runtime_outside_the_scope_answers_as_evidence_only() {
    let revision = Revision::default();
    let mut java = LanguageJava::new();
    let mut jvm = PlatformJvm::new();
    runtime_class(
        &mut jvm,
        revision,
        "java.lang.String",
        JvmAccessLevel::Public,
    );
    let asker = process(
        &mut java,
        &mut jvm,
        revision,
        "app/p/Test.java",
        "package p; class Test {}",
    );
    jvm.register_scopes(
        revision,
        asker.clone(),
        vec![JvmScope::of(vec![JvmContainer::Source(PathBuf::from(
            "app",
        ))])],
    );
    let query = JavaQuery::new(jvm.query_from(&asker, revision), &java);

    let JavaTypeResolution::Unresolved { invalid_candidates } =
        resolve(&java, &query, &asker, "String")
    else {
        panic!("a runtime image outside the scope must not resolve");
    };
    assert_eq!(invalid_candidates.len(), 1);
    assert!(invalid_candidates[0].has_invalidity(JavaTypeInvalidity::OutsideScope));
}
