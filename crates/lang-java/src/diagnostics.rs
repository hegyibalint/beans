use std::collections::HashSet;

use beans_core::analysis::diagnostic::{DiagnosticSeverity, Diagnostics};
use beans_platform_jvm::model::JvmSource;

use crate::accessibility::{JavaSite, is_accessible};
use crate::model::{
    JavaAccessLevel, JavaBodyId, JavaBodyNodeId, JavaBodyNodeKind, JavaDeclaration, JavaExpression,
    JavaFile, JavaImportKind,
};
use crate::query::JavaQuery;
use crate::resolution::{resolve_expression, resolve_variable_name};

/// Flags member accesses that resolve to a declaration JLS 26 §6.6.1 does not
/// let this site reach. Resolution stays permissive on purpose: navigating to
/// the thing you may not touch is more useful than pretending it is missing,
/// so the check runs here rather than inside `resolve_expression`.
pub fn access_diagnostics(
    source: &JvmSource,
    file: &JavaFile,
    query: &JavaQuery,
) -> Vec<Diagnostics> {
    let mut diagnostics = Vec::new();

    for (body_index, body) in file.bodies.iter().enumerate() {
        for (node_index, node) in body.nodes.iter().enumerate() {
            let JavaBodyNodeKind::Expression(JavaExpression::FieldAccess { name, .. }) = &node.kind
            else {
                continue;
            };

            let from = JavaSite {
                source,
                file,
                scope: node.scope,
            };
            let targets = resolve_expression(
                source,
                file,
                JavaBodyId(body_index),
                JavaBodyNodeId(node_index),
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
                    let JavaDeclaration::Field(field) = &target_file.declarations[declaration.0]
                    else {
                        return None;
                    };
                    let declared = JavaSite {
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

fn level_word(level: JavaAccessLevel) -> &'static str {
    match level {
        JavaAccessLevel::Public => "public",
        JavaAccessLevel::Protected => "protected",
        JavaAccessLevel::Package => "package-private",
        JavaAccessLevel::Private => "private",
    }
}

fn owner_name(file: &JavaFile, scope: crate::model::JavaLexicalScopeId) -> String {
    file.enclosing_type_declaration(scope)
        .and_then(|declaration| file.declarations[declaration.0].name())
        .map(|name| name.text.clone())
        .unwrap_or_else(|| "<unknown>".to_string())
}

/// Flags bare name references that resolve to nothing. Deliberately shallow:
/// member lookups on a receiver and type names are never flagged — inheritance
/// and `java.lang` are not modeled yet, so those checks would guess.
pub fn unresolved_name_diagnostics(model: &JavaFile) -> Vec<Diagnostics> {
    // Static imports bring names we cannot model yet; rather than flagging
    // them, stay silent for the whole file.
    if model.imports.iter().any(|import| {
        matches!(
            import.kind,
            JavaImportKind::Static | JavaImportKind::StaticOnDemand
        )
    }) {
        return Vec::new();
    }

    let mut diagnostics = Vec::new();
    for body in &model.bodies {
        // A superclass hides inherited members from us; bare names inherited
        // through it would be false positives.
        let inherits = model.iter_scope_chain(body.scope).any(|(_, scope)| {
            scope
                .owner
                .is_some_and(|owner| match &model.declarations[owner.0] {
                    JavaDeclaration::Type(declaration) => declaration.superclass.is_some(),
                    _ => false,
                })
        });
        if inherits {
            continue;
        }

        // Receivers of field accesses and calls may be type names
        // (`Bar.asd`); `java.lang` is not modeled, so never flag them.
        let mut receivers = HashSet::new();
        for node in &body.nodes {
            let JavaBodyNodeKind::Expression(expression) = &node.kind else {
                continue;
            };
            match expression {
                JavaExpression::FieldAccess { receiver, .. } => {
                    receivers.insert(receiver.0);
                }
                JavaExpression::MethodCall {
                    receiver: Some(receiver),
                    ..
                } => {
                    receivers.insert(receiver.0);
                }
                _ => {}
            }
        }

        for (index, node) in body.nodes.iter().enumerate() {
            let JavaBodyNodeKind::Expression(JavaExpression::NameRef { name }) = &node.kind else {
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
    use super::*;
    use crate::parser::JavaParser;

    fn parse(contents: &str) -> JavaFile {
        JavaParser::new().parse(contents)
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
