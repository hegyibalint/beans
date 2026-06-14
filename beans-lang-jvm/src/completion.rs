//! Neutral completion result types.
//!
//! Per ADR-0002 / ADR-0020 / backlog #025 the LSP-shaped completion
//! item (with formatted `detail` strings, parameter lists shaped like
//! the LSP wire) belongs in `beans-lsp`, not in the core library. The
//! core's responsibility is the *neutral* answer to "what's visible at
//! this cursor" — a list of candidates with enough information for any
//! consumer (the LSP, a CLI, a batch analyzer) to format on its own.
//!
//! The fixture harness asserts on these neutral types so it stays a
//! `beans-core`-only consumer per ADR-0020.

use crate::Fqn;
use crate::SymbolKind;
use beans_core::graph::NodeId;

/// One completion candidate — what one symbol would contribute to a
/// completion list. Producers fill these in from a graph walk; the
/// LSP-shaped formatter in `beans-lsp` adapts them into wire shapes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionCandidate {
    pub name: String,
    pub kind: SymbolKind,
    pub fqn: Fqn,
    /// The graph node this candidate references. Stable for the
    /// duration of one query; not preserved across rebuilds (per
    /// ADR-0007). Consumers that want a durable identity use the
    /// FQN.
    pub node_id: NodeId,
}

/// Thin wrapper around `Vec<CompletionCandidate>` with the query
/// methods spec tests use. The inner vec is private so the public
/// surface stays the query API ([`has`](Self::has), [`get`](Self::get),
/// [`count`](Self::count), [`names`](Self::names),
/// [`iter`](Self::iter)); producers in `beans-core` construct via
/// [`CompletionCandidates::default`] and append through internal
/// helpers when the completion engine lands. Today the only producer
/// is the fixture harness's empty stub.
#[derive(Debug, Default)]
pub struct CompletionCandidates(Vec<CompletionCandidate>);

impl CompletionCandidates {
    /// Is a candidate with this name and kind offered?
    pub fn has(&self, name: &str, kind: SymbolKind) -> bool {
        self.0.iter().any(|c| c.name == name && c.kind == kind)
    }

    /// Get the candidate with this name and kind. Panics with a clear
    /// message if missing — useful in test assertions.
    pub fn get(&self, name: &str, kind: SymbolKind) -> &CompletionCandidate {
        self.0
            .iter()
            .find(|c| c.name == name && c.kind == kind)
            .unwrap_or_else(|| {
                let available: Vec<_> = self
                    .0
                    .iter()
                    .map(|c| format!("{} ({:?})", c.name, c.kind))
                    .collect();
                panic!(
                    "completion candidate '{}' ({:?}) not found.\nAvailable items: {:?}",
                    name, kind, available
                );
            })
    }

    /// How many candidates of this kind?
    pub fn count(&self, kind: SymbolKind) -> usize {
        self.0.iter().filter(|c| c.kind == kind).count()
    }

    /// Sorted names of all candidates of a given kind.
    pub fn names(&self, kind: SymbolKind) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .0
            .iter()
            .filter(|c| c.kind == kind)
            .map(|c| c.name.as_str())
            .collect();
        names.sort();
        names
    }

    /// Iterator over all candidates, in insertion order.
    pub fn iter(&self) -> std::slice::Iter<'_, CompletionCandidate> {
        self.0.iter()
    }
}
