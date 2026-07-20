use beans_core::storage::Revision;
use beans_platform_jvm::{
    PlatformJvm,
    model::{JvmQualifiedName, JvmSource},
};

use crate::{
    LanguageJava,
    model::{
        JavaDeclaration, JavaDeclarationId, JavaFile, JavaIdentifier, JavaImport, JavaImportKind,
        JavaLexicalScopeId, JavaName,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaTypeTarget {
    Java {
        source: JvmSource,
        declaration: JavaDeclarationId,
    },
    Jvm(JvmQualifiedName),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaTypeResolution {
    Resolved(JavaTypeTarget),
    Ambiguous(Vec<JavaTypeTarget>),
    Unresolved,
}

pub fn resolve_type_name(
    name: &JavaName,
    source: &JvmSource,
    file: &JavaFile,
    current_lexical_scope_id: JavaLexicalScopeId,
    revision: Revision,
    _jvm: &PlatformJvm,
    java: &LanguageJava,
) -> JavaTypeResolution {
    // Qualified names follow a separate path that classifies their prefix and resolves members.
    let JavaName::Simple(name) = name else {
        todo!("resolve qualified type names");
    };

    // Stage 1. Resolve declarations through the lexical scope chain, nearest scope first.
    let lexical = resolve_lexical_type_name(name, source, file, current_lexical_scope_id);
    if !matches!(lexical, JavaTypeResolution::Unresolved) {
        return lexical;
    }

    // Stage 2. Resolve single-type and single-static imports.
    let exact_import = resolve_exact_imports(name, file, revision, java);
    if !matches!(exact_import, JavaTypeResolution::Unresolved) {
        return exact_import;
    }

    // Stage 3. Resolve top-level types in the current package.
    let same_package = resolve_from_same_package(name, file, revision, java);
    if !matches!(same_package, JavaTypeResolution::Unresolved) {
        return same_package;
    }

    // Stage 4. Resolve ordinary on-demand imports, static on-demand imports, and java.lang.

    // Stage 5. Resolve module imports.

    // Stage 6. Search accessible declarations for import suggestions.

    todo!("resolve stages 4 through 6");
}

fn resolve_lexical_type_name(
    name: &JavaIdentifier,
    source: &JvmSource,
    file: &JavaFile,
    current_lexical_scope_id: JavaLexicalScopeId,
) -> JavaTypeResolution {
    for (_, scope) in file.lexical_scope_chain(current_lexical_scope_id) {
        let candidates = scope
            .declarations
            .iter()
            .copied()
            .filter_map(|declaration_id| {
                let declaration = file.declarations.get(declaration_id.0)?;
                let declaration_name = match declaration {
                    JavaDeclaration::Type(declaration) => declaration.name.as_ref(),
                    JavaDeclaration::TypeParameter(declaration) => declaration.name.as_ref(),
                    _ => None,
                }?;

                (declaration_name.text == name.text).then(|| JavaTypeTarget::Java {
                    source: source.clone(),
                    declaration: declaration_id,
                })
            });
        let resolution = classify_candidates(candidates);
        if !matches!(resolution, JavaTypeResolution::Unresolved) {
            return resolution;
        }
    }

    JavaTypeResolution::Unresolved
}

fn resolve_exact_imports(
    name: &JavaIdentifier,
    file: &JavaFile,
    revision: Revision,
    java: &LanguageJava,
) -> JavaTypeResolution {
    if file.imports.iter().any(|import| {
        import.kind == JavaImportKind::Static && exact_import_introduces_name(import, name)
    }) {
        todo!("resolve single-static type imports");
    }

    let matching_imports = file
        .imports
        .iter()
        .filter(|import| import.kind == JavaImportKind::Type)
        .filter(|import| exact_import_introduces_name(import, name));

    let imported_java_files = matching_imports.flat_map(|import| {
        java.file_models_at(revision)
            .filter(move |(_, imported_file)| {
                let Some(type_segments) = imported_file.strip_package(&import.name) else {
                    return false;
                };
                !type_segments.is_empty()
            })
            .map(move |(source, file)| (import, source, file))
    });

    let candidates = imported_java_files
        .filter_map(|(import, source, imported_file)| {
            let type_segments = imported_file.strip_package(&import.name)?;
            let [type_name] = type_segments else {
                let top_level_name = type_segments.first()?;
                let imports_member_type = imported_file
                    .iter_declarations(&imported_file.top_level_declarations)
                    .any(|(_, declaration)| {
                        let JavaDeclaration::Type(declaration) = declaration else {
                            return false;
                        };

                        declaration
                            .name
                            .as_ref()
                            .is_some_and(|name| name.text == top_level_name.text)
                    });
                if imports_member_type {
                    todo!("resolve single-type imports of member types");
                }

                return None;
            };

            Some((source, imported_file, type_name))
        })
        .flat_map(|(source, imported_file, type_name)| {
            imported_file
                .top_level_declarations
                .iter()
                .copied()
                .filter_map(move |declaration_id| {
                    let JavaDeclaration::Type(declaration) =
                        imported_file.declarations.get(declaration_id.0)?
                    else {
                        return None;
                    };
                    let declaration_name = declaration.name.as_ref()?;

                    (declaration_name.text == type_name.text).then(|| JavaTypeTarget::Java {
                        source: source.clone(),
                        declaration: declaration_id,
                    })
                })
        });

    classify_candidates(candidates)
}

fn exact_import_introduces_name(import: &JavaImport, name: &JavaIdentifier) -> bool {
    import
        .name
        .segments()
        .last()
        .is_some_and(|imported| imported.text == name.text)
}

fn resolve_from_same_package(
    name: &JavaIdentifier,
    file: &JavaFile,
    revision: Revision,
    java: &LanguageJava,
) -> JavaTypeResolution {
    let candidates = java
        .file_models_at(revision)
        // Keep files in the same package
        .filter(|(_, model)| package_names_match(model.package.as_ref(), file.package.as_ref()))
        // Find top level declarations that match
        .flat_map(|(source, model)| {
            model
                .iter_declarations(&model.top_level_declarations)
                .filter_map(move |(declaration_id, declaration)| {
                    let JavaDeclaration::Type(declaration) = declaration else {
                        return None;
                    };
                    let declaration_name = declaration.name.as_ref()?;

                    (declaration_name.text == name.text).then(|| JavaTypeTarget::Java {
                        source: source.clone(),
                        declaration: declaration_id,
                    })
                })
        });

    classify_candidates(candidates)
}

fn package_names_match(left: Option<&JavaName>, right: Option<&JavaName>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => {
            let left_segments = left.segments();
            let right_segments = right.segments();
            if left_segments.len() != right_segments.len() {
                return false;
            }

            for index in 0..left_segments.len() {
                if left_segments[index].text != right_segments[index].text {
                    return false;
                }
            }

            true
        }
        _ => false,
    }
}

fn classify_candidates(candidates: impl IntoIterator<Item = JavaTypeTarget>) -> JavaTypeResolution {
    let mut distinct = Vec::new();
    for candidate in candidates {
        if !distinct.contains(&candidate) {
            distinct.push(candidate);
        }
    }

    match distinct.len() {
        0 => JavaTypeResolution::Unresolved,
        1 => JavaTypeResolution::Resolved(distinct.pop().unwrap()),
        _ => JavaTypeResolution::Ambiguous(distinct),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use beans_core::model::Span;

    use super::*;
    use crate::{model::JavaTypeDeclaration, parser::JavaParser};

    fn identifier(text: &str) -> JavaIdentifier {
        JavaIdentifier {
            text: text.into(),
            span: Span {
                start: 0,
                end: text.len(),
            },
        }
    }

    fn source(path: &str) -> JvmSource {
        JvmSource::SourceFile {
            path: PathBuf::from(path),
        }
    }

    fn type_declaration(
        file: &JavaFile,
        declaration_id: JavaDeclarationId,
    ) -> &JavaTypeDeclaration {
        let JavaDeclaration::Type(declaration) = &file.declarations[declaration_id.0] else {
            panic!("expected a type declaration");
        };
        declaration
    }

    fn type_in_scope(
        file: &JavaFile,
        scope_id: JavaLexicalScopeId,
        name: &str,
    ) -> JavaDeclarationId {
        file.lexical_scopes[scope_id.0]
            .declarations
            .iter()
            .copied()
            .find(|declaration_id| {
                type_declaration(file, *declaration_id)
                    .name
                    .as_ref()
                    .is_some_and(|identifier| identifier.text == name)
            })
            .unwrap()
    }

    fn file_model<'java>(
        java: &'java LanguageJava,
        revision: Revision,
        source: &JvmSource,
    ) -> &'java JavaFile {
        java.file_models_at(revision)
            .find_map(|(candidate, file)| (candidate == source).then_some(file))
            .unwrap()
    }

    fn process(
        java: &mut LanguageJava,
        jvm: &mut PlatformJvm,
        revision: Revision,
        path: &str,
        contents: &str,
    ) -> JvmSource {
        let source = source(path);
        java.process(source.clone(), revision, jvm, contents);
        source
    }

    #[test]
    fn lexical_resolution_prefers_the_innermost_scope() {
        let mut parser = JavaParser::new();
        let file = parser.parse("class Outer { class X {} class Inner { class X {} } }");
        let outer = file.top_level_declarations[0];
        let outer_scope = type_declaration(&file, outer).body_scope;
        let inner = type_in_scope(&file, outer_scope, "Inner");
        let inner_scope = type_declaration(&file, inner).body_scope;
        let inner_x = type_in_scope(&file, inner_scope, "X");
        let source = source("Outer.java");

        assert_eq!(
            resolve_lexical_type_name(&identifier("X"), &source, &file, inner_scope),
            JavaTypeResolution::Resolved(JavaTypeTarget::Java {
                source,
                declaration: inner_x,
            })
        );
    }

    #[test]
    fn lexical_resolution_continues_to_the_parent_scope() {
        let mut parser = JavaParser::new();
        let file = parser.parse("class Outer { class X {} class Inner {} }");
        let outer = file.top_level_declarations[0];
        let outer_scope = type_declaration(&file, outer).body_scope;
        let outer_x = type_in_scope(&file, outer_scope, "X");
        let inner = type_in_scope(&file, outer_scope, "Inner");
        let inner_scope = type_declaration(&file, inner).body_scope;
        let source = source("Outer.java");

        assert_eq!(
            resolve_lexical_type_name(&identifier("X"), &source, &file, inner_scope),
            JavaTypeResolution::Resolved(JavaTypeTarget::Java {
                source,
                declaration: outer_x,
            })
        );
    }

    #[test]
    fn same_package_resolves_a_top_level_type_by_package_spelling() {
        let revision = Revision::default();
        let mut java = LanguageJava::new();
        let mut jvm = PlatformJvm::new();
        let resolved_source = process(
            &mut java,
            &mut jvm,
            revision,
            "p/X.java",
            "/* shift the package span */ package p; class X {}",
        );
        let current_source = process(
            &mut java,
            &mut jvm,
            revision,
            "p/Test.java",
            "package p; class Test {}",
        );
        let resolved_declaration =
            file_model(&java, revision, &resolved_source).top_level_declarations[0];
        let current_file = file_model(&java, revision, &current_source);

        assert_eq!(
            resolve_from_same_package(&identifier("X"), current_file, revision, &java),
            JavaTypeResolution::Resolved(JavaTypeTarget::Java {
                source: resolved_source,
                declaration: resolved_declaration,
            })
        );
    }

    #[test]
    fn same_package_ignores_a_type_from_another_package() {
        let revision = Revision::default();
        let mut java = LanguageJava::new();
        let mut jvm = PlatformJvm::new();
        process(
            &mut java,
            &mut jvm,
            revision,
            "p/X.java",
            "package p; class X {}",
        );
        let current_source = process(
            &mut java,
            &mut jvm,
            revision,
            "q/Test.java",
            "package q; class Test {}",
        );
        let current_file = file_model(&java, revision, &current_source);

        assert_eq!(
            resolve_from_same_package(&identifier("X"), current_file, revision, &java),
            JavaTypeResolution::Unresolved
        );
    }

    #[test]
    fn same_package_resolves_a_type_from_the_unnamed_package() {
        let revision = Revision::default();
        let mut java = LanguageJava::new();
        let mut jvm = PlatformJvm::new();
        let resolved_source = process(&mut java, &mut jvm, revision, "X.java", "class X {}");
        let current_source = process(&mut java, &mut jvm, revision, "Test.java", "class Test {}");
        let resolved_declaration =
            file_model(&java, revision, &resolved_source).top_level_declarations[0];
        let current_file = file_model(&java, revision, &current_source);

        assert_eq!(
            resolve_from_same_package(&identifier("X"), current_file, revision, &java),
            JavaTypeResolution::Resolved(JavaTypeTarget::Java {
                source: resolved_source,
                declaration: resolved_declaration,
            })
        );
    }

    #[test]
    fn same_package_collects_candidates_from_every_matching_file() {
        let revision = Revision::default();
        let mut java = LanguageJava::new();
        let mut jvm = PlatformJvm::new();
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
        let first_declaration =
            file_model(&java, revision, &first_source).top_level_declarations[0];
        let second_declaration =
            file_model(&java, revision, &second_source).top_level_declarations[0];
        let current_file = file_model(&java, revision, &current_source);

        let JavaTypeResolution::Ambiguous(candidates) =
            resolve_from_same_package(&identifier("X"), current_file, revision, &java)
        else {
            panic!("expected ambiguous same-package types");
        };

        assert_eq!(candidates.len(), 2);
        assert!(candidates.contains(&JavaTypeTarget::Java {
            source: first_source,
            declaration: first_declaration,
        }));
        assert!(candidates.contains(&JavaTypeTarget::Java {
            source: second_source,
            declaration: second_declaration,
        }));
    }

    #[test]
    #[should_panic(expected = "resolve single-static type imports")]
    fn matching_single_static_imports_fail_loudly() {
        let revision = Revision::default();
        let mut java = LanguageJava::new();
        let mut jvm = PlatformJvm::new();
        let current_source = process(
            &mut java,
            &mut jvm,
            revision,
            "q/Test.java",
            "package q; import static p.Outer.Inner; class Test {}",
        );
        let current_file = file_model(&java, revision, &current_source);

        resolve_exact_imports(&identifier("Inner"), current_file, revision, &java);
    }

    #[test]
    #[should_panic(expected = "resolve single-type imports of member types")]
    fn matching_member_type_imports_fail_loudly() {
        let revision = Revision::default();
        let mut java = LanguageJava::new();
        let mut jvm = PlatformJvm::new();
        process(
            &mut java,
            &mut jvm,
            revision,
            "p/Outer.java",
            "package p; class Outer { class Inner {} }",
        );
        let current_source = process(
            &mut java,
            &mut jvm,
            revision,
            "q/Test.java",
            "package q; import p.Outer.Inner; class Test {}",
        );
        let current_file = file_model(&java, revision, &current_source);

        resolve_exact_imports(&identifier("Inner"), current_file, revision, &java);
    }

    #[test]
    #[should_panic(expected = "resolve stages 4 through 6")]
    fn unresolved_names_fail_at_the_unimplemented_stages() {
        let revision = Revision::default();
        let mut java = LanguageJava::new();
        let mut jvm = PlatformJvm::new();
        let current_source = process(&mut java, &mut jvm, revision, "Test.java", "class Test {}");
        let current_file = file_model(&java, revision, &current_source);

        resolve_type_name(
            &JavaName::Simple(identifier("Missing")),
            &current_source,
            current_file,
            current_file.compilation_unit_scope,
            revision,
            &jvm,
            &java,
        );
    }

    #[test]
    fn exact_import_resolves_a_java_top_level_type() {
        let revision = Revision::default();
        let mut java = LanguageJava::new();
        let mut jvm = PlatformJvm::new();
        let imported_source = process(
            &mut java,
            &mut jvm,
            revision,
            "p/X.java",
            "package p; class X {}",
        );
        let importing_source = process(
            &mut java,
            &mut jvm,
            revision,
            "q/Test.java",
            "package q; import p.X; class Test {}",
        );
        let imported_declaration =
            file_model(&java, revision, &imported_source).top_level_declarations[0];
        let importing_file = file_model(&java, revision, &importing_source);

        assert_eq!(
            resolve_exact_imports(&identifier("X"), importing_file, revision, &java),
            JavaTypeResolution::Resolved(JavaTypeTarget::Java {
                source: imported_source,
                declaration: imported_declaration,
            })
        );
    }

    #[test]
    fn exact_import_does_not_skip_an_intermediate_name_segment() {
        let revision = Revision::default();
        let mut java = LanguageJava::new();
        let mut jvm = PlatformJvm::new();
        process(
            &mut java,
            &mut jvm,
            revision,
            "p/Inner.java",
            "package p; class Inner {}",
        );
        let importing_source = process(
            &mut java,
            &mut jvm,
            revision,
            "q/Test.java",
            "package q; import p.Outer.Inner; class Test {}",
        );
        let importing_file = file_model(&java, revision, &importing_source);

        assert_eq!(
            resolve_exact_imports(&identifier("Inner"), importing_file, revision, &java),
            JavaTypeResolution::Unresolved
        );
    }

    #[test]
    fn exact_import_uses_the_file_package_as_the_type_boundary() {
        let revision = Revision::default();
        let mut java = LanguageJava::new();
        let mut jvm = PlatformJvm::new();
        let imported_source = process(
            &mut java,
            &mut jvm,
            revision,
            "p/Outer/Inner.java",
            "package p.Outer; class Inner {}",
        );
        let importing_source = process(
            &mut java,
            &mut jvm,
            revision,
            "q/Test.java",
            "package q; import p.Outer.Inner; class Test {}",
        );
        let imported_declaration =
            file_model(&java, revision, &imported_source).top_level_declarations[0];
        let importing_file = file_model(&java, revision, &importing_source);

        assert_eq!(
            resolve_exact_imports(&identifier("Inner"), importing_file, revision, &java),
            JavaTypeResolution::Resolved(JavaTypeTarget::Java {
                source: imported_source,
                declaration: imported_declaration,
            })
        );
    }

    #[test]
    fn duplicate_exact_imports_are_deduplicated() {
        let revision = Revision::default();
        let mut java = LanguageJava::new();
        let mut jvm = PlatformJvm::new();
        let imported_source = process(
            &mut java,
            &mut jvm,
            revision,
            "p/X.java",
            "package p; class X {}",
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
            resolve_exact_imports(&identifier("X"), importing_file, revision, &java),
            JavaTypeResolution::Resolved(JavaTypeTarget::Java {
                source: imported_source,
                declaration: imported_declaration,
            })
        );
    }

    #[test]
    fn distinct_exact_imports_are_ambiguous() {
        let revision = Revision::default();
        let mut java = LanguageJava::new();
        let mut jvm = PlatformJvm::new();
        let p_source = process(
            &mut java,
            &mut jvm,
            revision,
            "p/X.java",
            "package p; class X {}",
        );
        let r_source = process(
            &mut java,
            &mut jvm,
            revision,
            "r/X.java",
            "package r; class X {}",
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

        let JavaTypeResolution::Ambiguous(candidates) =
            resolve_exact_imports(&identifier("X"), importing_file, revision, &java)
        else {
            panic!("expected ambiguous exact imports");
        };

        assert_eq!(candidates.len(), 2);
        assert!(candidates.contains(&JavaTypeTarget::Java {
            source: p_source,
            declaration: p_declaration,
        }));
        assert!(candidates.contains(&JavaTypeTarget::Java {
            source: r_source,
            declaration: r_declaration,
        }));
    }
}
