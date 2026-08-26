use std::collections::HashSet;

use beans_core::analysis::diagnostic::{DiagnosticSeverity, Diagnostics};
use beans_platform_jvm as jvm;

use crate::accessibility::{Site, is_accessible};
use crate::model;
use crate::query::Query;
use crate::resolution::{
    TypeInvalidity, resolve_expression, resolve_type_name, resolve_variable_name,
};

/// JLS 26 §6.5.5.1 makes a simple type name a compile-time error unless one
/// declaration is in scope. Report the narrower case where Beans discovered a
/// matching declaration but the host compilation described by §7.3 cannot
/// observe its source.
pub fn type_scope_diagnostics(
    source: &jvm::model::Source,
    file: &model::File,
    query: &Query,
) -> Vec<Diagnostics> {
    file.declarations
        .iter()
        .flat_map(|declaration| written_types(declaration).map(move |ty| (declaration, ty)))
        .filter_map(|(declaration, type_ref)| {
            if type_ref.primitive {
                return None;
            }

            let resolution = resolve_type_name(
                &type_ref.name,
                source,
                file,
                declaration.declaring_scope(),
                query,
            );
            resolution
                .has_invalidity(TypeInvalidity::OutsideScope)
                .then(|| Diagnostics {
                    span: type_ref.name.span(),
                    severity: DiagnosticSeverity::Error,
                    code: "type-outside-scope",
                    message: format!(
                        "type {} is outside the current compilation scope",
                        type_ref.name.dotted()
                    ),
                })
        })
        .collect()
}

/// Every type a declaration names in source: the one it is annotated with, and
/// for a type declaration each name in its `extends` and `implements` clauses.
/// §8.1.4 and §9.1.3 make a supertype a type reference like any other, so the
/// same question is worth asking of it.
fn written_types(declaration: &model::Declaration) -> impl Iterator<Item = &model::TypeRef> {
    let supertypes = match declaration {
        model::Declaration::Type(declaration) => Some(declaration.supertypes()),
        _ => None,
    };

    declaration
        .type_ref()
        .into_iter()
        .chain(supertypes.into_iter().flatten().map(|(_, ty)| ty))
}

/// Flags member accesses that resolve to a declaration JLS 26 §6.6.1 does not
/// let this site reach. Resolution stays permissive on purpose: navigating to
/// the thing you may not touch is more useful than pretending it is missing,
/// so the check runs here rather than inside `resolve_expression`.
pub fn access_diagnostics(
    source: &jvm::model::Source,
    file: &model::File,
    query: &Query,
) -> Vec<Diagnostics> {
    let mut diagnostics = Vec::new();

    for (body_index, body) in file.bodies.iter().enumerate() {
        for (node_index, node) in body.nodes.iter().enumerate() {
            let model::BodyNodeKind::Expression(model::Expression::FieldAccess { name, .. }) =
                &node.kind
            else {
                continue;
            };

            let from = Site {
                source,
                file,
                scope: node.scope,
            };
            let targets = resolve_expression(
                source,
                file,
                model::BodyId(body_index),
                model::BodyNodeId(node_index),
                query,
            );

            // Nothing resolved is `cannot-find-symbol`'s business, and one
            // reachable target is enough to make the access legal.
            if targets.is_empty() {
                continue;
            }
            let unreachable: Vec<_> = targets
                .iter()
                .filter_map(|(target_source, declaration)| {
                    let target_file = query.model_of(target_source)?;
                    let model::Declaration::Field(field) = &target_file.declarations[declaration.0]
                    else {
                        return None;
                    };
                    let declared = Site {
                        source: target_source,
                        file: target_file,
                        scope: field.declaring_scope,
                    };
                    let level = field.access?.level;
                    (!is_accessible(field.access, &declared, &from))
                        .then(|| (level, owner_name(target_file, field.declaring_scope)))
                })
                .collect();

            if unreachable.len() != targets.len() {
                continue;
            }
            let Some((level, owner)) = unreachable.into_iter().next() else {
                continue;
            };

            diagnostics.push(Diagnostics {
                span: name.span,
                severity: DiagnosticSeverity::Error,
                code: "inaccessible-member",
                message: format!(
                    "{} has {} access in {}",
                    name.text,
                    level_word(level),
                    owner
                ),
            });
        }
    }

    diagnostics
}

fn level_word(level: model::AccessLevel) -> &'static str {
    match level {
        model::AccessLevel::Public => "public",
        model::AccessLevel::Protected => "protected",
        model::AccessLevel::Package => "package-private",
        model::AccessLevel::Private => "private",
    }
}

fn owner_name(file: &model::File, scope: crate::model::LexicalScopeId) -> String {
    file.enclosing_type_declaration(scope)
        .and_then(|declaration| file.declarations[declaration.0].name())
        .map(|name| name.text.clone())
        .unwrap_or_else(|| "<unknown>".to_string())
}

/// Flags bare name references that resolve to nothing. Deliberately shallow:
/// member lookups on a receiver and type names are never flagged — inheritance
/// is not modeled, and a type name reaches `java.lang` only once a project has
/// named a JDK, so those checks would guess.
pub fn unresolved_name_diagnostics(model: &model::File) -> Vec<Diagnostics> {
    // Static imports bring names we cannot model yet; rather than flagging
    // them, stay silent for the whole file.
    if model.imports.iter().any(|import| {
        matches!(
            import.kind,
            model::ImportKind::Static | model::ImportKind::StaticOnDemand
        )
    }) {
        return Vec::new();
    }

    let mut diagnostics = Vec::new();
    for body in &model.bodies {
        // A supertype hides inherited members from us; bare names inherited
        // through one would be false positives. §9.3 makes an interface's
        // fields constants a class reaches by their simple name, so
        // `implements` counts here exactly as `extends` does.
        let inherits = model.iter_scope_chain(body.scope).any(|(_, scope)| {
            scope
                .owner
                .is_some_and(|owner| match &model.declarations[owner.0] {
                    model::Declaration::Type(declaration) => {
                        declaration.supertypes().next().is_some()
                    }
                    _ => false,
                })
        });
        if inherits {
            continue;
        }

        // Receivers of field accesses and calls may be type names
        // (`System.out`), which reach `java.lang` only once a project has named
        // a JDK, so never flag them.
        let mut receivers = HashSet::new();
        for node in &body.nodes {
            let model::BodyNodeKind::Expression(expression) = &node.kind else {
                continue;
            };
            match expression {
                model::Expression::FieldAccess { receiver, .. } => {
                    receivers.insert(receiver.0);
                }
                model::Expression::MethodCall {
                    receiver: Some(receiver),
                    ..
                } => {
                    receivers.insert(receiver.0);
                }
                _ => {}
            }
        }

        for (index, node) in body.nodes.iter().enumerate() {
            let model::BodyNodeKind::Expression(model::Expression::NameRef { name }) = &node.kind
            else {
                continue;
            };
            if receivers.contains(&index) {
                continue;
            }
            if resolve_variable_name(model, name, node.scope).is_empty() {
                diagnostics.push(Diagnostics {
                    span: node.span,
                    severity: DiagnosticSeverity::Error,
                    code: "cannot-find-symbol",
                    message: format!("cannot find symbol: {}", name.text),
                });
            }
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use beans_core::{language::LanguageProcessing, storage::Revision};
    use beans_platform_jvm as jvm;

    use super::*;
    use crate::{Language, parser::Parser};

    fn parse(contents: &str) -> model::File {
        Parser::new().parse(contents)
    }

    fn source(path: &str) -> jvm::model::Source {
        jvm::model::Source::SourceFile {
            path: PathBuf::from(path),
        }
    }

    fn process(
        java: &mut Language,
        jvm: &mut jvm::Platform,
        revision: Revision,
        path: &str,
        contents: &str,
    ) -> jvm::model::Source {
        let source = source(path);
        java.process(source.clone(), revision, jvm, contents);
        source
    }

    #[test]
    fn flags_a_known_type_outside_the_compilation_scope() {
        let revision = Revision::default();
        let mut java = Language::new();
        let mut jvm = jvm::Platform::new();
        process(
            &mut java,
            &mut jvm,
            revision,
            "test/p/X.java",
            "package p; public class X {}",
        );
        let contents = "package p; class Test { void use(X target) {} }";
        let current = process(&mut java, &mut jvm, revision, "main/p/Test.java", contents);
        jvm.register_scopes(
            revision,
            current.clone(),
            vec![jvm::query::Scope::of(vec![jvm::query::Container::Source(
                PathBuf::from("main"),
            )])],
        );
        let file = java.model_at(&current, revision).unwrap();
        let query = Query::new(jvm.query_from(&current, revision), &java);

        let diagnostics = type_scope_diagnostics(&current, file, &query);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "type-outside-scope");
        assert_eq!(
            diagnostics[0].message,
            "type X is outside the current compilation scope"
        );
        let start = contents.find("X target").unwrap();
        assert_eq!(diagnostics[0].span.start.0, start);
        assert_eq!(diagnostics[0].span.end.0, start + 1);
    }

    /// §8.1.5's `implements` names a type like any other, so it is asked the
    /// same question. Written with `implements` rather than `extends` because
    /// only one of the two clauses is new: the superclass reached this check
    /// through `Declaration::type_ref`, and the interface list never did.
    #[test]
    fn flags_a_supertype_outside_the_compilation_scope() {
        let revision = Revision::default();
        let mut java = Language::new();
        let mut jvm = jvm::Platform::new();
        process(
            &mut java,
            &mut jvm,
            revision,
            "test/p/X.java",
            "package p; public interface X {}",
        );
        let contents = "package p; class Test implements X {}";
        let current = process(&mut java, &mut jvm, revision, "main/p/Test.java", contents);
        jvm.register_scopes(
            revision,
            current.clone(),
            vec![jvm::query::Scope::of(vec![jvm::query::Container::Source(
                PathBuf::from("main"),
            )])],
        );
        let file = java.model_at(&current, revision).unwrap();
        let query = Query::new(jvm.query_from(&current, revision), &java);

        let diagnostics = type_scope_diagnostics(&current, file, &query);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "type-outside-scope");
        let start = contents.find("X {}").unwrap();
        assert_eq!(diagnostics[0].span.start.0, start);
        assert_eq!(diagnostics[0].span.end.0, start + 1);
    }

    #[test]
    fn an_inaccessible_type_has_no_scope_diagnostic() {
        let revision = Revision::default();
        let mut java = Language::new();
        let mut jvm = jvm::Platform::new();
        process(
            &mut java,
            &mut jvm,
            revision,
            "q/X.java",
            "package q; class X {}",
        );
        let current = process(
            &mut java,
            &mut jvm,
            revision,
            "p/Test.java",
            "package p; import q.X; class Test { X field; }",
        );
        let file = java.model_at(&current, revision).unwrap();
        let query = Query::new(jvm.query_from(&current, revision), &java);

        assert!(type_scope_diagnostics(&current, file, &query).is_empty());
    }

    #[test]
    fn a_type_absent_from_the_lake_has_no_scope_diagnostic() {
        let revision = Revision::default();
        let mut java = Language::new();
        let mut jvm = jvm::Platform::new();
        let current = process(
            &mut java,
            &mut jvm,
            revision,
            "p/Test.java",
            "package p; class Test { Missing field; }",
        );
        let file = java.model_at(&current, revision).unwrap();
        let query = Query::new(jvm.query_from(&current, revision), &java);

        assert!(type_scope_diagnostics(&current, file, &query).is_empty());
    }

    #[test]
    fn flags_an_unresolvable_name() {
        let file = parse("class A {\n    void m() {\n        int d = e;\n    }\n}\n");

        let diagnostics = unresolved_name_diagnostics(&file);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "cannot-find-symbol");
        assert_eq!(
            diagnostics[0].span,
            beans_core::model::OffsetSpan {
                start: beans_core::model::Offset(41),
                end: beans_core::model::Offset(42),
            }
        );
        assert_eq!(diagnostics[0].severity, DiagnosticSeverity::Error);
    }

    #[test]
    fn resolvable_names_are_quiet() {
        let file = parse(
            "class A {\n    int a;\n    void b(int c) {\n        int d = c;\n        this.a = d;\n        b(d);\n    }\n}\n",
        );

        assert!(unresolved_name_diagnostics(&file).is_empty());
    }

    #[test]
    fn a_superclass_suppresses_the_body() {
        let file =
            parse("class A extends Base {\n    void m() {\n        inherited = 1;\n    }\n}\n");

        assert!(unresolved_name_diagnostics(&file).is_empty());
    }

    #[test]
    fn static_imports_suppress_the_file() {
        let file = parse(
            "import static p.Outer.CONST;\nclass A {\n    void m() {\n        int d = CONST;\n    }\n}\n",
        );

        assert!(unresolved_name_diagnostics(&file).is_empty());
    }

    #[test]
    fn a_receiver_name_ref_is_never_flagged() {
        let file = parse("class A {\n    void m() {\n        int d = System.value;\n    }\n}\n");

        assert!(unresolved_name_diagnostics(&file).is_empty());
    }
}
