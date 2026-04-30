//! LSP-shaped completion item and the adapter from the neutral
//! `beans_core::CompletionCandidate`.
//!
//! Per ADR-0020 LSP-shaped output (formatted `detail` strings,
//! parameter lists shaped like the LSP wire) lives in `beans-lsp`,
//! not in the core library. The core surfaces a neutral
//! [`CompletionCandidate`]; this module's [`CompletionItem`] adds the
//! formatting and [`to_completion_item`] is the adapter. A future
//! mapping into `lsp_types::CompletionItem` lives alongside the
//! request handler when completion lands in the actor.
//!
//! Today the adapter only consumes [`NodePayload::Java`]; Kotlin,
//! Scala, Groovy, and Clojure will add arms when those languages come
//! online. Bytecode-loaded `NodePayload::Jvm(...)` payloads (e.g.,
//! classpath-driven completions) are tracked in backlog #034.
//!
//! Today the LSP doesn't actually compute completions — the actor's
//! handlers don't list a completion request and the fixture's
//! `complete_default` returns an empty candidate list. Both swap into
//! these types when completion is implemented; the contract is
//! already in place so the swap doesn't churn the consumer surface.

use beans_core::languages::java::JavaNodePayload;
use beans_core::payload::NodePayload;
use beans_core::{CompletionCandidate, SymbolKind};

/// One formal parameter rendered for a completion `detail`. Carries
/// the source name, formatted type, and the varargs flag so
/// `String... names` renders correctly (rather than `String[] names`,
/// which is structurally true but not what a Java consumer expects).
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionParam {
    pub name: String,
    pub ty: String,
    pub is_varargs: bool,
}

/// LSP-shaped completion item, ready to format into the wire type.
/// Carries the same name + kind + FQN as a [`CompletionCandidate`]
/// plus the LSP-formatted `detail` string and parameter list.
///
/// Currently unused at runtime — the actor's request handlers do not
/// list a completion handler yet, and the fixture's
/// `complete_default` returns an empty candidate list. The unit tests
/// at the bottom of this module exercise the adapter so it stays
/// honest while the full path catches up.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub name: String,
    pub kind: SymbolKind,
    pub fqn: String,
    pub return_type: String,
    pub params: Vec<CompletionParam>,
    pub detail: String,
}

/// Adapt a [`CompletionCandidate`] into a [`CompletionItem`] using
/// the resolved node's payload. Returns the candidate's name +
/// kind + FQN with formatted strings derived from the payload's
/// declared parameters / return type.
///
/// Looking up the payload for `candidate.node_id` is the consumer's
/// responsibility (it requires graph access); this function takes the
/// payload directly so it can be called from any thread that holds a
/// payload reference.
#[allow(dead_code)]
pub fn to_completion_item(
    candidate: &CompletionCandidate,
    payload: &NodePayload,
) -> CompletionItem {
    let mut return_type = String::new();
    let mut params: Vec<CompletionParam> = Vec::new();

    if let NodePayload::Java(java) = payload {
        match java {
            JavaNodePayload::Method(m) => {
                return_type = m.return_type.to_string();
                params = m
                    .parameters
                    .iter()
                    .map(|p| CompletionParam {
                        name: p.name.clone(),
                        ty: p.param_type.to_string(),
                        is_varargs: p.is_varargs,
                    })
                    .collect();
            }
            JavaNodePayload::Constructor(c) => {
                params = c
                    .parameters
                    .iter()
                    .map(|p| CompletionParam {
                        name: p.name.clone(),
                        ty: p.param_type.to_string(),
                        is_varargs: p.is_varargs,
                    })
                    .collect();
            }
            JavaNodePayload::Field(f) => {
                return_type = f.field_type.to_string();
            }
            // TODO: AnnotationElement when @Anno(<cur>) completion lands;
            // Type, EnumConstant, Parameter, Package are intentional no-ops.
            _ => {}
        }
    }

    let detail = build_detail(&candidate.kind, &return_type, &params);
    CompletionItem {
        name: candidate.name.clone(),
        kind: candidate.kind,
        fqn: candidate.fqn.to_string(),
        return_type,
        params,
        detail,
    }
}

#[allow(dead_code)]
fn build_detail(
    kind: &SymbolKind,
    return_type: &str,
    params: &[CompletionParam],
) -> String {
    match kind {
        SymbolKind::Method | SymbolKind::Constructor => {
            let param_str: Vec<String> = params
                .iter()
                .map(|p| {
                    if p.is_varargs {
                        // Walker stores `String... xs` as
                        // `is_varargs=true` with `param_type=String[]`
                        // (per JLS the varargs slot is array-shaped at
                        // the JVM level). Render the source-level form.
                        let base = p.ty.strip_suffix("[]").unwrap_or(&p.ty);
                        format!("{base}... {}", p.name)
                    } else {
                        format!("{} {}", p.ty, p.name)
                    }
                })
                .collect();
            if return_type.is_empty() {
                format!("({})", param_str.join(", "))
            } else {
                format!("({}) -> {}", param_str.join(", "), return_type)
            }
        }
        SymbolKind::Field | SymbolKind::EnumConstant => return_type.to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beans_core::Registries;
    use beans_core::graph::Graph;
    use beans_core::jvm::Fqn;
    use beans_core::languages::java;
    use beans_core::payload::NodePayload;

    /// Helper: parse `source`, integrate, and return both the graph
    /// and the inserted NodeIds so individual tests can pluck the
    /// payload they care about.
    fn fixture(
        source: &str,
    ) -> (Graph<NodePayload>, Vec<beans_core::graph::NodeId>) {
        let mut graph: Graph<NodePayload> = Graph::new();
        let registries = Registries::new();
        let parsed =
            java::parse_java_to_graph(std::path::Path::new("Test.java"), source);
        let inserted = java::integrate(&mut graph, &registries, parsed);
        (graph, inserted)
    }

    fn find_named<'g>(
        graph: &'g Graph<NodePayload>,
        ids: &[beans_core::graph::NodeId],
        name: &str,
    ) -> (beans_core::graph::NodeId, &'g NodePayload) {
        let id = ids
            .iter()
            .copied()
            .find(|&id| match graph.get(id).map(|n| &n.payload) {
                Some(NodePayload::Java(j)) => {
                    j.header().is_some_and(|h| h.name == name)
                }
                _ => false,
            })
            .unwrap_or_else(|| panic!("no Java payload named '{name}'"));
        let payload = &graph.get(id).unwrap().payload;
        (id, payload)
    }

    #[test]
    fn method_candidate_formats_detail_and_params() {
        let (graph, inserted) = fixture(
            "package com.example;\npublic class Service { public String greet(String who) { return null; } }\n",
        );
        let (id, payload) = find_named(&graph, &inserted, "greet");

        let candidate = CompletionCandidate {
            name: "greet".to_string(),
            kind: SymbolKind::Method,
            fqn: Fqn::new("com.example.Service.greet"),
            node_id: id,
        };
        let item = to_completion_item(&candidate, payload);

        assert_eq!(item.name, "greet");
        assert_eq!(item.kind, SymbolKind::Method);
        assert_eq!(item.return_type, "String");
        assert_eq!(item.params.len(), 1);
        assert_eq!(item.params[0].name, "who");
        assert_eq!(item.params[0].ty, "String");
        assert!(!item.params[0].is_varargs);
        assert_eq!(item.detail, "(String who) -> String");
    }

    #[test]
    fn varargs_method_renders_with_ellipsis() {
        // `String... names` should render as `String... names` in the
        // detail, not `String[] names`. The walker stores the
        // varargs slot as `param_type=String[]` with
        // `is_varargs=true` (per JLS the JVM-level type is array-
        // shaped); the adapter restores the source-level form.
        let (graph, inserted) = fixture(
            "package com.example;\npublic class V { public void greet(String... names) {} }\n",
        );
        let (id, payload) = find_named(&graph, &inserted, "greet");

        let candidate = CompletionCandidate {
            name: "greet".to_string(),
            kind: SymbolKind::Method,
            fqn: Fqn::new("com.example.V.greet"),
            node_id: id,
        };
        let item = to_completion_item(&candidate, payload);

        assert_eq!(item.params.len(), 1);
        assert!(item.params[0].is_varargs);
        assert_eq!(item.detail, "(String... names) -> void");
    }

    #[test]
    fn void_return_method_includes_void_in_detail() {
        // `void` is the documented return-type rendering for methods
        // with no return value (per the existing fixture's hover
        // formatting); the adapter passes it through verbatim.
        let (graph, inserted) = fixture(
            "package com.example;\npublic class V { public void run() {} }\n",
        );
        let (id, payload) = find_named(&graph, &inserted, "run");

        let candidate = CompletionCandidate {
            name: "run".to_string(),
            kind: SymbolKind::Method,
            fqn: Fqn::new("com.example.V.run"),
            node_id: id,
        };
        let item = to_completion_item(&candidate, payload);

        assert_eq!(item.return_type, "void");
        assert_eq!(item.detail, "() -> void");
    }

    #[test]
    fn constructor_detail_omits_arrow() {
        // Constructors don't have a declared return type — the
        // walker's `JavaConstructorNode` carries only parameters.
        // `build_detail` renders constructors as `(params)` without
        // the arrow.
        let (graph, inserted) = fixture(
            "package com.example;\npublic class V { public V(int n) {} }\n",
        );
        // The class is named V; the constructor is also named V. Find
        // the Constructor payload specifically.
        let id = inserted
            .iter()
            .copied()
            .find(|&id| {
                matches!(
                    graph.get(id).map(|n| &n.payload),
                    Some(NodePayload::Java(JavaNodePayload::Constructor(_)))
                )
            })
            .expect("constructor should be present");
        let payload = &graph.get(id).unwrap().payload;

        let candidate = CompletionCandidate {
            name: "V".to_string(),
            kind: SymbolKind::Constructor,
            fqn: Fqn::new("com.example.V.V"),
            node_id: id,
        };
        let item = to_completion_item(&candidate, payload);

        assert_eq!(item.return_type, "");
        assert_eq!(item.params.len(), 1);
        assert_eq!(item.params[0].name, "n");
        assert_eq!(item.params[0].ty, "int");
        assert_eq!(item.detail, "(int n)");
    }
}
