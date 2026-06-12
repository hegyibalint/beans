//! Java diagnostic rules.
//!
//! Per ADR-0017 each language module owns its rules; the engine merely
//! provides the [`Rule`] trait and the file-level dispatch in
//! [`crate::diagnostics`]. Per ADR-0029 slice 1 ships:
//!
//! - [`AbstractMethodWithBody`] — `abstract` method declarations may not
//!   carry a `{ ... }` body (JLS §8.4.3.1). Reads the
//!   [`JavaMethodNode::has_body`] flag the walker writes.
//! - [`UnusedImport`] — single-type imports that no `JavaTypeUseNode`
//!   in the file resolves to. Wildcard and static imports stay
//!   unflagged in slice 1 (they need body-slice use sites that
//!   ADR-0029 explicitly defers).

use std::collections::HashSet;

use crate::diagnostics::{Diagnostic, DiagnosticSeverity, Rule, RuleContext};
use crate::languages::java::keys::JavaSymbolKey;
use crate::languages::java::payload::JavaNodePayload;
use crate::languages::java::syntax::Import;
use crate::payload::NodePayload;
use crate::Modifier;

/// JLS §8.4.3.1: a method declared `abstract` may not also be defined
/// with a body. Implemented as a per-node walk: every
/// [`JavaMethodNode`](crate::languages::java::JavaMethodNode) in the
/// file whose modifiers contain [`Modifier::Abstract`] and whose
/// `has_body` flag is set produces one error diagnostic at the
/// declaration's location.
pub struct AbstractMethodWithBody;

impl Rule for AbstractMethodWithBody {
    fn code(&self) -> &'static str {
        "abstract-method-with-body"
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        let mut out = Vec::new();
        for (_id, node) in ctx.graph.iter() {
            let NodePayload::Java(JavaNodePayload::Method(m)) = &node.payload else {
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
                    code: Some(self.code().to_string()),
                });
            }
        }
        out
    }
}

/// JLS §7.5.1: a single-type-import declaration introduces a name that
/// must be used somewhere in the compilation unit. The
/// `unused-import` rule walks every [`JavaTypeUseNode`] in the file,
/// resolves each through `java_symbols`, and flags single-imports
/// whose target FQN never appears in the resolved set.
///
/// Slice-1 limits per ADR-0029:
/// - Wildcard imports (`import com.foo.*;`) are not flagged. Without
///   body-slice use sites we can't tell if a wildcard contributed to
///   resolving any name; flagging would produce false positives.
/// - Static imports (`import static com.foo.Util.MAX;`) are not
///   flagged. They bind member names that show up only in body-level
///   `JavaIdentifierReadNode` / `JavaMethodCallNode` nodes — which
///   slice 1 doesn't emit yet.
pub struct UnusedImport;

impl Rule for UnusedImport {
    fn code(&self) -> &'static str {
        "unused-import"
    }

    fn check(&self, ctx: &RuleContext<'_>) -> Vec<Diagnostic> {
        if ctx.java_imports.is_empty() {
            return Vec::new();
        }

        // Collect every FQN that some JavaTypeUseNode in this file
        // resolves to. Resolution: try each candidate FQN against
        // `java_symbols`; the first non-empty provider list wins.
        let mut used_fqns: HashSet<String> = HashSet::new();
        for (_id, node) in ctx.graph.iter() {
            let NodePayload::Java(JavaNodePayload::TypeUse(t)) = &node.payload else {
                continue;
            };
            if t.header.location.file != ctx.file {
                continue;
            }
            for fqn in &t.header.candidate_fqns {
                let key = JavaSymbolKey::new(fqn.clone());
                if !ctx.registries.java_symbols.providers(&key).is_empty() {
                    used_fqns.insert(fqn.as_str().to_string());
                    break;
                }
            }
        }

        let mut out = Vec::new();
        for imp in ctx.java_imports {
            match imp {
                Import::Single(fqn, location) => {
                    if !used_fqns.contains(fqn) {
                        out.push(Diagnostic {
                            location: location.clone(),
                            severity: DiagnosticSeverity::Warning,
                            message: format!("Unused import: `{}`.", fqn),
                            code: Some(self.code().to_string()),
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
}

/// Rule list for `.java` files. Per ADR-0017 this is a plain function,
/// not a runtime registry — adding/removing a rule is a one-line change
/// here.
pub fn rules() -> Vec<Box<dyn Rule>> {
    vec![Box::new(AbstractMethodWithBody), Box::new(UnusedImport)]
}
