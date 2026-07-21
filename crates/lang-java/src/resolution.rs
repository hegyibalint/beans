use beans_core::language::NavigationTarget;
use beans_core::storage::Revision;
use beans_platform_jvm::{
    PlatformJvm,
    model::{JvmQualifiedName, JvmSource},
};

use crate::{
    LanguageJava,
    model::{
        JavaBodyId, JavaDeclaration, JavaDeclarationId, JavaEntityId, JavaExpression,
        JavaExpressionId, JavaFile, JavaIdentifier, JavaImport, JavaImportKind, JavaLexicalScopeId,
        JavaName, JavaNamespace, JavaTypeRef,
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
    // Qualified names follow a separate path that classifies their prefix and
    // resolves members; not implemented yet.
    let JavaName::Simple(name) = name else {
        return JavaTypeResolution::Unresolved;
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
    // Not implemented yet: absence of the later stages means unresolved,
    // never a guess.
    JavaTypeResolution::Unresolved
}

fn resolve_lexical_type_name(
    name: &JavaIdentifier,
    source: &JvmSource,
    file: &JavaFile,
    current_lexical_scope_id: JavaLexicalScopeId,
) -> JavaTypeResolution {
    for (_, scope) in file.iter_scope_chain(current_lexical_scope_id) {
        let candidates = scope
            .declarations
            .iter()
            .copied()
            .filter_map(|declaration_id| {
                let declaration = file.declarations.get(declaration_id.0)?;
                let declaration_name = match declaration {
                    JavaDeclaration::Type(declaration) => declaration.name.as_ref(),
                    JavaDeclaration::TypeParameter(declaration) => Some(&declaration.name),
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
    let matching_imports = file
        .imports
        .iter()
        .filter(|import| import.kind == JavaImportKind::Type)
        .filter(|import| exact_import_introduces_name(import, name));

    let imported_java_files = matching_imports.flat_map(|import| {
        java.iter_file_models_at(revision)
            .filter(move |(_, imported_file)| {
                let Some(type_segments) = imported_file.strip_package(&import.name) else {
                    return false;
                };
                !type_segments.is_empty()
            })
            .map(move |(source, file)| (import, source, file))
    });

    let candidates = imported_java_files.filter_map(|(import, source, imported_file)| {
        let type_segments = imported_file.strip_package(&import.name)?;
        let (first, member_path) = type_segments.split_first()?;

        let mut current =
            imported_file
                .top_level_declarations
                .iter()
                .copied()
                .find(|declaration_id| {
                    let JavaDeclaration::Type(declaration) =
                        &imported_file.declarations[declaration_id.0]
                    else {
                        return false;
                    };
                    declaration
                        .name
                        .as_ref()
                        .is_some_and(|name| name.text == first.text)
                })?;
        for segment in member_path {
            current = find_member(imported_file, current, segment, JavaNamespace::Type)
                .into_iter()
                .next()?;
        }

        Some(JavaTypeTarget::Java {
            source: source.clone(),
            declaration: current,
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
        .iter_file_models_at(revision)
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

/// The members of a type with a matching name in the given namespace.
/// No inheritance yet: only the type's own body scope is searched.
fn find_member(
    file: &JavaFile,
    type_declaration: JavaDeclarationId,
    name: &JavaIdentifier,
    namespace: JavaNamespace,
) -> Vec<JavaDeclarationId> {
    let JavaDeclaration::Type(declaration) = &file.declarations[type_declaration.0] else {
        return Vec::new();
    };

    file.lexical_scopes[declaration.body_scope.0]
        .declarations
        .iter()
        .copied()
        .filter(|member_id| {
            let member = &file.declarations[member_id.0];
            member.namespace() == namespace && member.name().is_some_and(|n| n.text == name.text)
        })
        .collect()
}

/// Resolves the occurrence at `offset` to the declarations it refers to.
/// This is the go-to-declaration entry point.
pub fn resolve_occurrence_at(
    source: &JvmSource,
    file: &JavaFile,
    offset: usize,
    revision: Revision,
    jvm: &PlatformJvm,
    java: &LanguageJava,
) -> Vec<NavigationTarget<JvmSource>> {
    let Some((_, entity)) = file.position_index.tightest_at(offset) else {
        return Vec::new();
    };

    let targets = match entity {
        JavaEntityId::Declaration(declaration) => vec![(source.clone(), declaration)],
        JavaEntityId::TypeRef(owner) => {
            let declaration = &file.declarations[owner.0];
            let Some(type_ref) = declaration.type_ref() else {
                return Vec::new();
            };
            resolve_type_reference(
                source,
                file,
                type_ref,
                declaration.declaring_scope(),
                revision,
                jvm,
                java,
            )
        }
        JavaEntityId::Expression(body, expression) => {
            resolve_expression(source, file, body, expression, revision, jvm, java)
        }
        JavaEntityId::Statement(..) | JavaEntityId::Scope(..) | JavaEntityId::Import(..) => {
            Vec::new()
        }
    };

    targets
        .iter()
        .filter_map(|(target_source, declaration_id)| {
            let target_file = model_of(java, revision, target_source)?;
            let span = target_file.declarations[declaration_id.0].name_span()?;
            Some(NavigationTarget {
                source: target_source.clone(),
                span,
            })
        })
        .collect()
}

fn resolve_expression(
    source: &JvmSource,
    file: &JavaFile,
    body_id: JavaBodyId,
    expression_id: JavaExpressionId,
    revision: Revision,
    jvm: &PlatformJvm,
    java: &LanguageJava,
) -> Vec<(JvmSource, JavaDeclarationId)> {
    let body = &file.bodies[body_id.0];
    match body.expression(expression_id) {
        JavaExpression::NameRef { name } => resolve_variable_name(file, name)
            .into_iter()
            .map(|declaration| (source.clone(), declaration))
            .collect(),
        JavaExpression::This => {
            let span = body.expression_span(expression_id);
            file.scope_containing(span.start)
                .and_then(|scope| file.enclosing_type_declaration(scope))
                .map(|declaration| vec![(source.clone(), declaration)])
                .unwrap_or_default()
        }
        JavaExpression::FieldAccess { receiver, name } => {
            let Some((class_source, class)) =
                resolve_receiver_class(source, file, body_id, *receiver, revision, jvm, java)
            else {
                return Vec::new();
            };
            let Some(class_file) = model_of(java, revision, &class_source) else {
                return Vec::new();
            };
            find_member(class_file, class, name, JavaNamespace::Variable)
                .into_iter()
                .map(|member| (class_source.clone(), member))
                .collect()
        }
        JavaExpression::MethodCall { receiver, name, .. } => {
            let receiver_class = match receiver {
                Some(receiver) => {
                    resolve_receiver_class(source, file, body_id, *receiver, revision, jvm, java)
                }
                None => file
                    .scope_containing(name.span.start)
                    .and_then(|scope| file.enclosing_type_declaration(scope))
                    .map(|declaration| (source.clone(), declaration)),
            };
            let Some((class_source, class)) = receiver_class else {
                return Vec::new();
            };
            let Some(class_file) = model_of(java, revision, &class_source) else {
                return Vec::new();
            };
            find_member(class_file, class, name, JavaNamespace::Method)
                .into_iter()
                .map(|member| (class_source.clone(), member))
                .collect()
        }
        JavaExpression::ObjectCreation { ty, .. } => {
            let Some(scope) = file.scope_containing(ty.span.start) else {
                return Vec::new();
            };
            resolve_type_reference(source, file, ty, scope, revision, jvm, java)
        }
        JavaExpression::Assign { .. } | JavaExpression::Literal => Vec::new(),
    }
}

/// The class through which member lookup for `expression` runs: the declared
/// type of the expression, or the type itself for static access (`Bar.asd`).
fn resolve_receiver_class(
    source: &JvmSource,
    file: &JavaFile,
    body_id: JavaBodyId,
    expression_id: JavaExpressionId,
    revision: Revision,
    jvm: &PlatformJvm,
    java: &LanguageJava,
) -> Option<(JvmSource, JavaDeclarationId)> {
    let body = &file.bodies[body_id.0];
    match body.expression(expression_id) {
        JavaExpression::This => {
            let span = body.expression_span(expression_id);
            let declaration = file
                .scope_containing(span.start)
                .and_then(|scope| file.enclosing_type_declaration(scope))?;
            Some((source.clone(), declaration))
        }
        JavaExpression::NameRef { name } => {
            if let Some(variable) = resolve_variable_name(file, name).into_iter().next() {
                let declaration = &file.declarations[variable.0];
                let type_ref = declaration.type_ref()?;
                return resolve_type_reference(
                    source,
                    file,
                    type_ref,
                    declaration.declaring_scope(),
                    revision,
                    jvm,
                    java,
                )
                .into_iter()
                .next();
            }

            // Not a variable: try a type name for static access (`Bar.asd`).
            let scope = file.scope_containing(name.span.start)?;
            match resolve_type_name(
                &JavaName::Simple(name.clone()),
                source,
                file,
                scope,
                revision,
                jvm,
                java,
            ) {
                JavaTypeResolution::Resolved(JavaTypeTarget::Java {
                    source,
                    declaration,
                }) => Some((source, declaration)),
                _ => None,
            }
        }
        JavaExpression::FieldAccess { receiver, name } => {
            let (class_source, class) =
                resolve_receiver_class(source, file, body_id, *receiver, revision, jvm, java)?;
            let class_file = model_of(java, revision, &class_source)?;
            let member = find_member(class_file, class, name, JavaNamespace::Variable)
                .into_iter()
                .next()?;
            let declaration = &class_file.declarations[member.0];
            resolve_type_reference(
                &class_source,
                class_file,
                declaration.type_ref()?,
                declaration.declaring_scope(),
                revision,
                jvm,
                java,
            )
            .into_iter()
            .next()
        }
        JavaExpression::MethodCall { receiver, name, .. } => {
            let receiver_class = match receiver {
                Some(receiver) => {
                    resolve_receiver_class(source, file, body_id, *receiver, revision, jvm, java)
                }
                None => file
                    .scope_containing(name.span.start)
                    .and_then(|scope| file.enclosing_type_declaration(scope))
                    .map(|declaration| (source.clone(), declaration)),
            }?;
            let (class_source, class) = receiver_class;
            let class_file = model_of(java, revision, &class_source)?;
            let member = find_member(class_file, class, name, JavaNamespace::Method)
                .into_iter()
                .next()?;
            let declaration = &class_file.declarations[member.0];
            resolve_type_reference(
                &class_source,
                class_file,
                declaration.type_ref()?,
                declaration.declaring_scope(),
                revision,
                jvm,
                java,
            )
            .into_iter()
            .next()
        }
        JavaExpression::ObjectCreation { ty, .. } => {
            let scope = file.scope_containing(ty.span.start)?;
            resolve_type_reference(source, file, ty, scope, revision, jvm, java)
                .into_iter()
                .next()
        }
        JavaExpression::Assign { .. } | JavaExpression::Literal => None,
    }
}

/// A syntactic type annotation resolved to its declaring class.
fn resolve_type_reference(
    source: &JvmSource,
    file: &JavaFile,
    type_ref: &JavaTypeRef,
    scope: JavaLexicalScopeId,
    revision: Revision,
    jvm: &PlatformJvm,
    java: &LanguageJava,
) -> Vec<(JvmSource, JavaDeclarationId)> {
    if type_ref.primitive {
        return Vec::new();
    }

    match resolve_type_name(&type_ref.name, source, file, scope, revision, jvm, java) {
        JavaTypeResolution::Resolved(JavaTypeTarget::Java {
            source,
            declaration,
        }) => vec![(source, declaration)],
        JavaTypeResolution::Ambiguous(targets) => targets
            .into_iter()
            .filter_map(|target| match target {
                JavaTypeTarget::Java {
                    source,
                    declaration,
                } => Some((source, declaration)),
                JavaTypeTarget::Jvm(_) => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// A bare name in expression position: locals, parameters, then fields,
/// nearest scope first. Always in-file.
pub(crate) fn resolve_variable_name(
    file: &JavaFile,
    name: &JavaIdentifier,
) -> Vec<JavaDeclarationId> {
    let Some(scope) = file.scope_containing(name.span.start) else {
        return Vec::new();
    };

    for (_, scope) in file.iter_scope_chain(scope) {
        let hits: Vec<JavaDeclarationId> = scope
            .declarations
            .iter()
            .copied()
            .filter(|declaration_id| {
                match &file.declarations[declaration_id.0] {
                    // JLS 6.3: a local's scope starts at its declarator.
                    JavaDeclaration::Local(declaration) => {
                        declaration.name.as_ref().is_some_and(|local| {
                            local.text == name.text && local.span.start <= name.span.start
                        })
                    }
                    JavaDeclaration::Parameter(_) | JavaDeclaration::Field(_) => file.declarations
                        [declaration_id.0]
                        .name()
                        .is_some_and(|candidate| candidate.text == name.text),
                    _ => false,
                }
            })
            .collect();
        if !hits.is_empty() {
            return hits;
        }
    }

    Vec::new()
}

pub(crate) fn model_of<'java>(
    java: &'java LanguageJava,
    revision: Revision,
    source: &JvmSource,
) -> Option<&'java JavaFile> {
    java.iter_file_models_at(revision)
        .find_map(|(candidate, file)| (candidate == source).then_some(file))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use beans_core::{language::LanguageProcessing, model::Span};

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
        model_of(java, revision, source).unwrap()
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
    fn static_imports_do_not_introduce_type_names_yet() {
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

        assert_eq!(
            resolve_exact_imports(&identifier("Inner"), current_file, revision, &java),
            JavaTypeResolution::Unresolved
        );
    }

    #[test]
    fn exact_import_resolves_a_member_type() {
        let revision = Revision::default();
        let mut java = LanguageJava::new();
        let mut jvm = PlatformJvm::new();
        let outer_source = process(
            &mut java,
            &mut jvm,
            revision,
            "p/Outer.java",
            "package p; class Outer { class Inner {} }",
        );
        let importing_source = process(
            &mut java,
            &mut jvm,
            revision,
            "q/Test.java",
            "package q; import p.Outer.Inner; class Test {}",
        );
        let outer_file = file_model(&java, revision, &outer_source);
        let outer_scope =
            type_declaration(outer_file, outer_file.top_level_declarations[0]).body_scope;
        let inner = type_in_scope(outer_file, outer_scope, "Inner");
        let importing_file = file_model(&java, revision, &importing_source);

        assert_eq!(
            resolve_exact_imports(&identifier("Inner"), importing_file, revision, &java),
            JavaTypeResolution::Resolved(JavaTypeTarget::Java {
                source: outer_source,
                declaration: inner,
            })
        );
    }

    #[test]
    fn unimplemented_stages_yield_unresolved() {
        let revision = Revision::default();
        let mut java = LanguageJava::new();
        let mut jvm = PlatformJvm::new();
        let current_source = process(&mut java, &mut jvm, revision, "Test.java", "class Test {}");
        let current_file = file_model(&java, revision, &current_source);

        assert_eq!(
            resolve_type_name(
                &JavaName::Simple(identifier("Missing")),
                &current_source,
                current_file,
                current_file.compilation_unit_scope,
                revision,
                &jvm,
                &java,
            ),
            JavaTypeResolution::Unresolved
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

    // The worked example from PLAN.md, with `B` in a second file so member
    // lookup crosses files. Offsets are load-bearing.
    const WORKED: &str = "class A {\n    int a;\n\n    void b(B c) {\n        int d = c.a;\n        this.a = d;\n        b(c);\n    }\n}\n";

    fn worked_fixture() -> (LanguageJava, PlatformJvm, Revision, JvmSource, JvmSource) {
        let revision = Revision::default();
        let mut java = LanguageJava::new();
        let mut jvm = PlatformJvm::new();
        let a = process(&mut java, &mut jvm, revision, "A.java", WORKED);
        let b = process(
            &mut java,
            &mut jvm,
            revision,
            "B.java",
            "class B {\n    int a;\n}\n",
        );
        (java, jvm, revision, a, b)
    }

    fn resolve_at(
        java: &LanguageJava,
        jvm: &PlatformJvm,
        revision: Revision,
        source: &JvmSource,
        offset: usize,
    ) -> Vec<NavigationTarget<JvmSource>> {
        let file = file_model(java, revision, source);
        resolve_occurrence_at(source, file, offset, revision, jvm, java)
    }

    #[test]
    fn occurrence_resolution_walks_the_worked_example() {
        let (java, jvm, revision, a_source, b_source) = worked_fixture();

        // (6) `c` in `c.a` → parameter c @ 35..36 in A.java
        let targets = resolve_at(&java, &jvm, revision, &a_source, 56);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].source, a_source);
        assert_eq!(targets[0].span, Span { start: 35, end: 36 });

        // (7) `a` in `c.a` → field a in B.java @ 18..19
        let targets = resolve_at(&java, &jvm, revision, &a_source, 58);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].source, b_source);
        assert_eq!(targets[0].span, Span { start: 18, end: 19 });

        // (8) `this` → class A @ 6..7
        let targets = resolve_at(&java, &jvm, revision, &a_source, 70);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].span, Span { start: 6, end: 7 });

        // (9) `a` in `this.a` → field a in A.java @ 18..19
        let targets = resolve_at(&java, &jvm, revision, &a_source, 74);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].span, Span { start: 18, end: 19 });

        // (10) `d` → local d @ 52..53
        let targets = resolve_at(&java, &jvm, revision, &a_source, 78);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].span, Span { start: 52, end: 53 });

        // (11) `b` in `b(c)` → method b @ 31..32
        let targets = resolve_at(&java, &jvm, revision, &a_source, 89);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].span, Span { start: 31, end: 32 });

        // (12) `c` argument → parameter c @ 35..36
        let targets = resolve_at(&java, &jvm, revision, &a_source, 91);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].span, Span { start: 35, end: 36 });

        // (3) parameter type `B` → class B in B.java @ 6..7
        let targets = resolve_at(&java, &jvm, revision, &a_source, 33);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].source, b_source);
        assert_eq!(targets[0].span, Span { start: 6, end: 7 });
    }

    #[test]
    fn a_local_is_not_visible_before_its_declarator() {
        let revision = Revision::default();
        let mut java = LanguageJava::new();
        let mut jvm = PlatformJvm::new();
        let contents = "class A {\n    void m() {\n        x = 1;\n        int x;\n    }\n}\n";
        let a = process(&mut java, &mut jvm, revision, "A.java", contents);
        // `x` at offset 33, used before the declarator at 52
        let targets = resolve_at(&java, &jvm, revision, &a, 33);
        assert!(targets.is_empty());
    }

    #[test]
    fn a_parameter_shadows_a_field() {
        let revision = Revision::default();
        let mut java = LanguageJava::new();
        let mut jvm = PlatformJvm::new();
        let contents = "class A {\n    int x;\n    void m(int x) {\n        x = 1;\n    }\n}\n";
        let a = process(&mut java, &mut jvm, revision, "A.java", contents);
        // `x` in the body @ 49 → the parameter @ 36..37, not the field @ 18..19
        let targets = resolve_at(&java, &jvm, revision, &a, 49);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].span, Span { start: 36, end: 37 });
    }
}
