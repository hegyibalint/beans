use super::*;

#[test]
fn duplicate_exact_imports_name_one_target() {
    let revision = Revision::default();
    let mut java = Language::new();
    let mut jvm = jvm::Platform::new();
    let imported_source = process(
        &mut java,
        &mut jvm,
        revision,
        "p/X.java",
        "package p; public class X {}",
    );
    let importing_source = process(
        &mut java,
        &mut jvm,
        revision,
        "q/Test.java",
        "package q; import p.X; import p.X; class Test {}",
    );
    let imported_declaration =
        file_model(&java, revision, &imported_source).top_level_declarations[0];
    let importing_file = file_model(&java, revision, &importing_source);

    assert_eq!(
        resolve_type_from_exact_imports(
            &identifier("X"),
            &importing_source,
            importing_file,
            &java_query(&java, &jvm, revision)
        ),
        TypeResolution::Resolved(TypeTarget::Parsed {
            source: imported_source,
            declaration: imported_declaration,
        })
    );
}

#[test]
fn distinct_exact_imports_leave_the_name_contested() {
    let revision = Revision::default();
    let mut java = Language::new();
    let mut jvm = jvm::Platform::new();
    let p_source = process(
        &mut java,
        &mut jvm,
        revision,
        "p/X.java",
        "package p; public class X {}",
    );
    let r_source = process(
        &mut java,
        &mut jvm,
        revision,
        "r/X.java",
        "package r; public class X {}",
    );
    let importing_source = process(
        &mut java,
        &mut jvm,
        revision,
        "q/Test.java",
        "package q; import p.X; import r.X; class Test {}",
    );
    let p_declaration = file_model(&java, revision, &p_source).top_level_declarations[0];
    let r_declaration = file_model(&java, revision, &r_source).top_level_declarations[0];
    let importing_file = file_model(&java, revision, &importing_source);

    let TypeResolution::Ambiguous(candidates) = resolve_type_from_exact_imports(
        &identifier("X"),
        &importing_source,
        importing_file,
        &java_query(&java, &jvm, revision),
    ) else {
        panic!("expected ambiguous exact imports");
    };

    assert_eq!(candidates.len(), 2);
    assert!(candidates.contains(&TypeTarget::Parsed {
        source: p_source,
        declaration: p_declaration,
    }));
    assert!(candidates.contains(&TypeTarget::Parsed {
        source: r_source,
        declaration: r_declaration,
    }));
}

#[test]
fn a_package_declaring_a_name_twice_offers_both_files() {
    let revision = Revision::default();
    let mut java = Language::new();
    let mut jvm = jvm::Platform::new();
    let first_source = process(
        &mut java,
        &mut jvm,
        revision,
        "p/First.java",
        "package p; class X {}",
    );
    let second_source = process(
        &mut java,
        &mut jvm,
        revision,
        "p/Second.java",
        "package p; class X {}",
    );
    let current_source = process(
        &mut java,
        &mut jvm,
        revision,
        "p/Test.java",
        "package p; class Test {}",
    );
    let first_declaration = file_model(&java, revision, &first_source).top_level_declarations[0];
    let second_declaration = file_model(&java, revision, &second_source).top_level_declarations[0];
    let current_file = file_model(&java, revision, &current_source);

    let TypeResolution::Ambiguous(candidates) = resolve_from_same_package(
        &identifier("X"),
        &current_source,
        current_file,
        &java_query(&java, &jvm, revision),
    ) else {
        panic!("expected ambiguous same-package types");
    };

    assert_eq!(candidates.len(), 2);
    assert!(candidates.contains(&TypeTarget::Parsed {
        source: first_source,
        declaration: first_declaration,
    }));
    assert!(candidates.contains(&TypeTarget::Parsed {
        source: second_source,
        declaration: second_declaration,
    }));
}
