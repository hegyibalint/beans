//! Neutral projections of [`NodePayload`] for consumers.
//!
//! Resolution and outline queries need a uniform `(kind, name, fqn,
//! location, modifiers)` view over a node regardless of which language
//! produced it. [`payload_view`] is that projection; [`PayloadView`] is
//! the borrowed result. [`DocSymbol`] is the language-neutral outline
//! node `document_symbols` returns — the LSP rim maps it onto the wire's
//! `DocumentSymbol`, keeping LSP-shaped formatting out of the facade
//! (ADR-0020).
//!
//! This module is gated on `feature = "java"`: today only Java payloads
//! carry declaration views (JVM-projection nodes are siblings of their
//! Java counterparts, and resolution always lands on the Java side, so
//! they project to `None`). When another vertical lands, its payload
//! arm joins [`payload_view`] and the gate widens.

use crate::NodePayload;
use crate::languages::java::{JavaNodePayload, JavaTypeKind};
use crate::{Location, Modifier, SymbolKind};

/// A uniform declaration view over a node payload: the fields every
/// consumer (go-to-definition, hover, outline, references) needs without
/// re-matching the payload union.
pub struct PayloadView<'a> {
    pub kind: SymbolKind,
    pub name: &'a str,
    pub fqn: &'a str,
    pub location: Option<&'a Location>,
    pub modifiers: &'a [Modifier],
}

/// Project a payload onto the neutral declaration view. JVM-projection
/// nodes and use-site nodes (imports, type uses) return `None`:
/// resolution lands on the *target* declaration, not on a projection or
/// a use site.
pub fn payload_view(payload: &NodePayload) -> Option<PayloadView<'_>> {
    let java = match payload {
        NodePayload::Java(j) => j,
        // JVM-projection nodes are not declaration views; resolution
        // always lands on the Java side. Matched explicitly (not `_`) so
        // a new `NodePayload` variant is a compile error here, not a
        // silent `None` (ADR-0030's closed unions).
        NodePayload::Jvm(_) => return None,
    };
    let view = match java {
        JavaNodePayload::Type(n) => {
            let kind = match n.kind {
                JavaTypeKind::Class => SymbolKind::Class,
                JavaTypeKind::Interface => SymbolKind::Interface,
                JavaTypeKind::Enum => SymbolKind::Enum,
                JavaTypeKind::Record => SymbolKind::Record,
                JavaTypeKind::Annotation => SymbolKind::Annotation,
            };
            PayloadView {
                kind,
                name: &n.header.name,
                fqn: n.header.fqn.as_str(),
                location: n.header.location.as_ref(),
                modifiers: &n.header.modifiers,
            }
        }
        JavaNodePayload::Method(n) => PayloadView {
            kind: SymbolKind::Method,
            name: &n.header.name,
            fqn: n.header.fqn.as_str(),
            location: n.header.location.as_ref(),
            modifiers: &n.header.modifiers,
        },
        JavaNodePayload::Constructor(n) => PayloadView {
            kind: SymbolKind::Constructor,
            name: &n.header.name,
            fqn: n.header.fqn.as_str(),
            location: n.header.location.as_ref(),
            modifiers: &n.header.modifiers,
        },
        JavaNodePayload::Field(n) => PayloadView {
            kind: SymbolKind::Field,
            name: &n.header.name,
            fqn: n.header.fqn.as_str(),
            location: n.header.location.as_ref(),
            modifiers: &n.header.modifiers,
        },
        // `EnumConstant` collapses to `Field` for spec-test stability;
        // backlog #032 tracks whether to surface it distinctly.
        JavaNodePayload::EnumConstant(n) => PayloadView {
            kind: SymbolKind::Field,
            name: &n.header.name,
            fqn: n.header.fqn.as_str(),
            location: n.header.location.as_ref(),
            modifiers: &n.header.modifiers,
        },
        JavaNodePayload::AnnotationElement(n) => PayloadView {
            kind: SymbolKind::Method,
            name: &n.header.name,
            fqn: n.header.fqn.as_str(),
            location: n.header.location.as_ref(),
            modifiers: &n.header.modifiers,
        },
        JavaNodePayload::Parameter(p) => PayloadView {
            kind: SymbolKind::Parameter,
            name: &p.name,
            fqn: "",
            location: None,
            modifiers: &[],
        },
        JavaNodePayload::Package(n) => PayloadView {
            kind: SymbolKind::Package,
            name: &n.header.name,
            fqn: n.header.fqn.as_str(),
            location: n.header.location.as_ref(),
            modifiers: &n.header.modifiers,
        },
        // Use-site nodes are not declaration views.
        JavaNodePayload::TypeUse(_) => return None,
        JavaNodePayload::Import(_) => return None,
    };
    Some(view)
}

/// A language-neutral outline node — the result of
/// [`Workspace::document_symbols`](crate::Workspace::document_symbols).
/// Mirrors the shape of an LSP `DocumentSymbol` without depending on the
/// wire types: the rim maps `kind` and `location` onto the protocol.
#[derive(Debug, Clone, PartialEq)]
pub struct DocSymbol {
    pub name: String,
    pub kind: SymbolKind,
    /// One-line signature detail (e.g. `(String) -> String` for a
    /// method, the field type for a field). `None` when there is
    /// nothing useful to show.
    pub detail: Option<String>,
    pub location: Option<Location>,
    pub children: Vec<DocSymbol>,
}

/// Render the one-line `detail` for an outline node: method signatures
/// and field types get a detail; everything else gets `None`.
pub(crate) fn doc_symbol_detail(payload: &NodePayload) -> Option<String> {
    match payload {
        NodePayload::Java(JavaNodePayload::Method(m)) => {
            let params: Vec<String> = m
                .parameters
                .iter()
                .map(|p| p.param_type.to_string())
                .collect();
            Some(format!("({}) -> {}", params.join(", "), m.return_type))
        }
        NodePayload::Java(JavaNodePayload::Field(f)) => Some(f.field_type.to_string()),
        // Other Java declarations carry no one-line detail; JVM
        // projections are never outline entries. Both arms explicit so a
        // new `NodePayload` variant must be handled here (ADR-0030).
        NodePayload::Java(_) => None,
        NodePayload::Jvm(_) => None,
    }
}

/// Whether a payload is a JVM-projection node. Outline building skips
/// these: a Java declaration and its JVM projection are siblings in the
/// graph, and only the Java side becomes a user-facing outline entry.
pub(crate) fn is_jvm_projection(payload: &NodePayload) -> bool {
    matches!(payload, NodePayload::Jvm(_))
}
