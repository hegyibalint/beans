//! Java diagnostic rules.
//!
//! Per ADR-0017 each vertical owns its rules as plain functions — no
//! central pipeline machinery, no rule registry. The `beans` facade's
//! `compute_diagnostics` dispatches `.java` files to [`check_file`];
//! adding or disabling a rule is a one-line change there.
//!
//! Rules are generic over the graph payload `P: AsJava` because the
//! payload union lives in the facade, above this crate. Per ADR-0029
//! slice 1 ships:
//!
//! - [`abstract_method_with_body`] — `abstract` method declarations may
//!   not carry a `{ ... }` body (JLS §8.4.3.1). Reads the
//!   [`JavaMethodNode::has_body`](crate::payload::JavaMethodNode::has_body)
//!   flag the walker writes.
//! - [`unused_import`] — single-type imports that no `JavaTypeUseNode`
//!   in the file resolves to. Wildcard and static imports stay
//!   unflagged in slice 1 (they need body-slice use sites that
//!   ADR-0029 explicitly defers).

use std::collections::HashSet;
use std::path::Path;

use beans_core::diagnostics::{Diagnostic, DiagnosticSeverity};
use beans_core::graph::Graph;
use beans_lang_jvm::payload::AsJvm;
use beans_lang_jvm::registries::JvmRegistries;
use beans_lang_jvm::Modifier;

use crate::keys::JavaSymbolKey;
use crate::payload::{AsJava, JavaNodePayload};
use crate::registries::JavaRegistries;
use crate::syntax::Import;

/// Read-only view the Java rules see. Borrowed from the engine; lives
/// only for the duration of the check call.
///
/// `imports` carries the file's `import` declarations as a side channel
/// until the `file://` root nodes of ADR-0029's modifiability axis land.
pub struct JavaRuleContext<'a, P> {
    pub graph: &'a Graph<P>,
    pub java: &'a JavaRegistries,
    pub jvm: &'a JvmRegistries,
    pub file: &'a Path,
    pub imports: &'a [Import],
}

/// Run every Java rule against `file` and merge the findings.
pub fn check_file<P: AsJava + AsJvm>(
    graph: &Graph<P>,
    java: &JavaRegistries,
    jvm: &JvmRegistries,
    file: &Path,
    imports: &[Import],
) -> Vec<Diagnostic> {
    let ctx = JavaRuleContext {
        graph,
        java,
        jvm,
        file,
        imports,
    };
    let mut out = abstract_method_with_body(&ctx);
    out.extend(unused_import(&ctx));
    out.extend(missing_import(&ctx));
    out
}

/// The dual of [`unused_import`]: a `JavaTypeUseNode` whose candidate
/// chain misses entirely, but whose simple name matches at least one
/// importable workspace type, warns at the use-site identifier.
///
/// Cause-framed and fix-gated: an unresolved name with no candidate
/// (e.g. a JDK type before jmod loading lands) stays silent — this is
/// "you forgot an import", not "reference not found". The gate widens
/// into a true reference-not-found rule once the JDK universe exists.
/// One diagnostic per occurrence (javac/IDE convention); applying the
/// fix at any occurrence clears all of them on the next recompute.
pub fn missing_import<P: AsJava + AsJvm>(ctx: &JavaRuleContext<'_, P>) -> Vec<Diagnostic> {
    const CODE: &str = "missing-import";
    let mut out = Vec::new();
    for (_id, node) in ctx.graph.iter() {
        let Some(JavaNodePayload::TypeUse(t)) = node.payload.as_java() else {
            continue;
        };
        if t.header.location.file != ctx.file {
            continue;
        }
        if crate::fixes::is_resolved(t, ctx.java) {
            continue;
        }
        let candidates =
            crate::fixes::import_candidates(&t.header.name, ctx.java, ctx.jvm, ctx.graph);
        if candidates.is_empty() {
            continue; // the gate: no fix, no diagnostic
        }
        let list = candidates
            .iter()
            .map(|f| format!("`{}`", f))
            .collect::<Vec<_>>()
            .join(", ");
        out.push(Diagnostic {
            location: t.header.location.clone(),
            severity: DiagnosticSeverity::Warning,
            message: format!(
                "Type `{}` is unresolved; importable candidate{}: {}.",
                t.header.name,
                if candidates.len() == 1 { "" } else { "s" },
                list
            ),
            code: Some(CODE.to_string()),
        });
    }
    out
}

/// JLS §8.4.3.1: a method declared `abstract` may not also be defined
/// with a body. Per-node walk: every Java method in the file whose
/// modifiers contain [`Modifier::Abstract`] and whose `has_body` flag
/// is set produces one error diagnostic at the declaration's location.
pub fn abstract_method_with_body<P: AsJava>(ctx: &JavaRuleContext<'_, P>) -> Vec<Diagnostic> {
    const CODE: &str = "abstract-method-with-body";
    let mut out = Vec::new();
    for (_id, node) in ctx.graph.iter() {
        let Some(JavaNodePayload::Method(m)) = node.payload.as_java() else {
            continue;
        };
        let Some(loc) = m.header.location.as_ref() else {
            continue;
        };
        if loc.file != ctx.file {
            continue;
        }
        if m.header.modifiers.contains(&Modifier::Abstract) && m.has_body {
            out.push(Diagnostic {
                location: loc.clone(),
                severity: DiagnosticSeverity::Error,
                message: format!(
                    "Method `{}` is declared `abstract` but has a body. \
                     Abstract methods may not have an implementation \
                     (JLS §8.4.3.1).",
                    m.header.name
                ),
                code: Some(CODE.to_string()),
            });
        }
    }
    out
}

/// JLS §7.5.1: a single-type-import declaration introduces a name that
/// must be used somewhere in the compilation unit. Walks every
/// `JavaTypeUseNode` in the file, resolves each through the Java
/// symbol registry, and flags single-imports whose target FQN never
/// appears in the resolved set.
///
/// Slice-1 limits per ADR-0029: wildcard and static imports are not
/// flagged (they need body-slice use sites).
pub fn unused_import<P: AsJava>(ctx: &JavaRuleContext<'_, P>) -> Vec<Diagnostic> {
    const CODE: &str = "unused-import";
    if ctx.imports.is_empty() {
        return Vec::new();
    }

    // Collect every FQN that some JavaTypeUseNode in this file
    // resolves to. Resolution: try each candidate FQN against the
    // Java symbols; the first non-empty provider list wins.
    let mut used_fqns: HashSet<String> = HashSet::new();
    for (_id, node) in ctx.graph.iter() {
        let Some(JavaNodePayload::TypeUse(t)) = node.payload.as_java() else {
            continue;
        };
        if t.header.location.file != ctx.file {
            continue;
        }
        for fqn in &t.header.candidate_fqns {
            let key = JavaSymbolKey::new(fqn.clone());
            if !ctx.java.symbols.providers(&key).is_empty() {
                used_fqns.insert(fqn.as_str().to_string());
                break;
            }
        }
    }

    let mut out = Vec::new();
    for imp in ctx.imports {
        match imp {
            Import::Single(fqn, location) => {
                if !used_fqns.contains(fqn) {
                    out.push(Diagnostic {
                        location: location.clone(),
                        severity: DiagnosticSeverity::Warning,
                        message: format!("Unused import: `{}`.", fqn),
                        code: Some(CODE.to_string()),
                    });
                }
            }
            Import::Wildcard(_, _) | Import::Static(_, _) => {
                // Slice 1: deferred — see ADR-0029.
            }
        }
    }
    out
}
