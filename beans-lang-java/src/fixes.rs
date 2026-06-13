//! Import quick fixes — the actionable half of the `missing-import`
//! diagnostic.
//!
//! Everything here is request-scoped and stateless (one-shot `Query`
//! semantics, no subscriptions): consumers re-derive fixes at request
//! time against the live graph, so a fix can never act on stale state
//! (the ADR-0028 rule, made structural). The LSP maps the returned
//! [`Fix`] values to `CodeAction`s; the test harness applies them
//! directly — same values, two appliers.

use std::collections::BTreeSet;
use std::path::Path;

use beans_core::fix::{Fix, SourceEdit};
use beans_core::graph::Graph;
use beans_core::primitives::Location;
use beans_lang_jvm::payload::AsJvm;
use beans_lang_jvm::registries::JvmRegistries;
use beans_lang_jvm::Fqn;

use crate::keys::JavaSymbolKey;
use crate::payload::{AsJava, JavaNodePayload, JavaTypeUseNode};
use crate::registries::JavaRegistries;

/// All importable workspace types named `name`, FQN-sorted and deduped.
///
/// Consults the ADR-0008 pair: the Java registry (native primary) and
/// the shared JVM type registry (fallback — a Java type and its
/// projection share an FQN and dedupe; bytecode-only types appear on
/// the JVM side alone once jmod loading lands). Only type declarations
/// are importable; `java.symbols` also holds methods and fields, which
/// are filtered out by payload kind.
pub fn import_candidates<P: AsJava + AsJvm>(
    name: &str,
    java: &JavaRegistries,
    jvm: &JvmRegistries,
    graph: &Graph<P>,
) -> Vec<Fqn> {
    let mut out: BTreeSet<String> = BTreeSet::new();

    for id in java.symbols.query_simple_name(name) {
        // java.symbols holds every declaration kind; only type
        // declarations are importable.
        if let Some(JavaNodePayload::Type(t)) =
            graph.get(id).and_then(|n| n.payload.as_java())
        {
            out.insert(t.header.fqn.as_str().to_string());
        }
    }

    for id in jvm.types.query_simple_name(name) {
        if let Some(jvm_node) = graph.get(id).and_then(|n| n.payload.as_jvm()) {
            if let Some(header) = jvm_node.header() {
                out.insert(header.fqn.as_str().to_string());
            }
        }
    }

    out.into_iter().map(Fqn::new).collect()
}

/// True iff the type use binds to something: its parser-time candidate
/// chain hits the Java symbol registry (ADR-0029 slice-1 resolution —
/// first FQN that hits wins).
pub fn is_resolved(t: &JavaTypeUseNode, java: &JavaRegistries) -> bool {
    t.header.candidate_fqns.iter().any(|fqn| {
        !java
            .symbols
            .providers(&JavaSymbolKey::new(fqn.clone()))
            .is_empty()
    })
}

/// The type-use node whose identifier span contains `(line, col)` in
/// `file`, if any. Linear over the graph — request-scoped lookup,
/// same posture as the simple-name scan.
pub fn type_use_at<'a, P: AsJava>(
    graph: &'a Graph<P>,
    file: &Path,
    line: u32,
    col: u32,
) -> Option<&'a JavaTypeUseNode> {
    for (_id, node) in graph.iter() {
        let Some(JavaNodePayload::TypeUse(t)) = node.payload.as_java() else {
            continue;
        };
        let loc = &t.header.location;
        if loc.file.as_ref() != file || !span_contains(loc, line, col) {
            continue;
        }
        return Some(t);
    }
    None
}

fn span_contains(loc: &Location, line: u32, col: u32) -> bool {
    let after_start = line > loc.start_line || (line == loc.start_line && col >= loc.start_col);
    let before_end = line < loc.end_line || (line == loc.end_line && col <= loc.end_col);
    after_start && before_end
}

/// Build the `Import '<fqn>'` fix for `file`.
///
/// v1 placement policy: one insertion directly after the package
/// statement, blank-line separated; top of file when packageless.
/// Sorting and grouping are organize-imports' job, later.
pub fn add_import_fix(file: &Path, source: &str, fqn: &Fqn) -> Fix {
    let package_line = source.lines().position(|l| {
        let t = l.trim_start();
        t.starts_with("package ") && t.trim_end().ends_with(';')
    });

    let (line, new_text) = match package_line {
        Some(n) => ((n + 1) as u32, format!("\nimport {};\n", fqn)),
        None => (0, format!("import {};\n\n", fqn)),
    };

    Fix {
        label: format!("Import '{}'", fqn),
        edits: vec![SourceEdit {
            location: Location {
                file: std::sync::Arc::from(file),
                start_line: line,
                start_col: 0,
                end_line: line,
                end_col: 0,
            },
            new_text,
        }],
    }
}

/// Quick fixes at a position: when it sits on an unresolved type use
/// that has importable candidates, one fix per candidate, FQN-sorted.
/// Empty otherwise — resolved uses and candidate-less names (e.g. JDK
/// types before jmod loading) offer nothing.
pub fn quick_fixes_at<P: AsJava + AsJvm>(
    graph: &Graph<P>,
    java: &JavaRegistries,
    jvm: &JvmRegistries,
    file: &Path,
    source: &str,
    line: u32,
    col: u32,
) -> Vec<Fix> {
    let Some(t) = type_use_at(graph, file, line, col) else {
        return Vec::new();
    };
    if is_resolved(t, java) {
        return Vec::new();
    }
    import_candidates(&t.header.name, java, jvm, graph)
        .iter()
        .map(|fqn| add_import_fix(file, source, fqn))
        .collect()
}
