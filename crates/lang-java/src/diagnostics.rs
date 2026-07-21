use std::collections::HashSet;

use beans_core::analysis::diagnostic::{DiagnosticSeverity, Diagnostics};

use crate::model::{JavaDeclaration, JavaExpression, JavaFile, JavaImportKind};
use crate::resolution::resolve_variable_name;

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
        let inherits = model.lexical_scope_chain(body.scope).any(|(_, scope)| {
            scope.owner.is_some_and(|owner| {
                match &model.declarations[owner.0] {
                    JavaDeclaration::Type(declaration) => declaration.superclass.is_some(),
                    _ => false,
                }
            })
        });
        if inherits {
            continue;
        }

        // Receivers of field accesses and calls may be type names
        // (`Bar.asd`); `java.lang` is not modeled, so never flag them.
        let mut receivers = HashSet::new();
        for expression in &body.expressions {
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

        for (index, expression) in body.expressions.iter().enumerate() {
            let JavaExpression::NameRef { name } = expression else {
                continue;
            };
            if receivers.contains(&index) {
                continue;
            }
            if resolve_variable_name(model, name).is_empty() {
                diagnostics.push(Diagnostics {
                    span: body.expression_spans[index],
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
        assert_eq!(diagnostics[0].span, beans_core::model::Span { start: 41, end: 42 });
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
        let file = parse("class A extends Base {\n    void m() {\n        inherited = 1;\n    }\n}\n");

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
