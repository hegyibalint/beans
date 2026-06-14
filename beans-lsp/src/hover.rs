//! Hover-text formatting for the LSP.
//!
//! Per ADR-0020 LSP-shaped output (markdown blobs, `lsp_types::Hover`)
//! lives in `beans-lsp`, not `beans-core`. The fixture harness
//! exercises hover assertions through its own minimal formatter; the
//! LSP carries a fuller implementation here that targets the wire
//! protocol.

use beans::NodePayload;
use beans::languages::java::{JavaNodePayload, JavaTypeKind};
use std::fmt::Write;

/// Build a markdown hover blob for a resolved Java payload. Returns
/// `None` for variants where there is nothing meaningful to show
/// (currently `Parameter`, which carries no header).
pub fn build_hover_text(payload: &NodePayload) -> Option<String> {
    let java = match payload {
        NodePayload::Java(j) => j,
        // JVM-projection nodes are siblings of their Java counterparts;
        // the resolver always lands on the Java side, so reaching this
        // arm should be rare. Fall through to a minimal hover.
        NodePayload::Jvm(_) => return None,
    };

    let mut out = String::new();
    match java {
        JavaNodePayload::Method(m) => {
            let tp = if m.type_parameters.is_empty() {
                String::new()
            } else {
                let names: Vec<&str> = m.type_parameters.iter().map(|t| t.name.as_str()).collect();
                format!("<{}>", names.join(", "))
            };
            let params: Vec<String> = m
                .parameters
                .iter()
                .map(|p| format!("{} {}", p.param_type, p.name))
                .collect();
            let _ = write!(
                out,
                "```java\n{}{} {}({})\n```\n\nmethod `{}`",
                tp,
                m.return_type,
                m.header.name,
                params.join(", "),
                m.header.fqn
            );
        }
        JavaNodePayload::Constructor(c) => {
            let params: Vec<String> = c
                .parameters
                .iter()
                .map(|p| format!("{} {}", p.param_type, p.name))
                .collect();
            let _ = write!(
                out,
                "```java\n{}({})\n```\n\nconstructor `{}`",
                c.header.name,
                params.join(", "),
                c.header.fqn
            );
        }
        JavaNodePayload::Field(f) => {
            let _ = write!(
                out,
                "```java\n{} {}\n```\n\nfield `{}`",
                f.field_type, f.header.name, f.header.fqn
            );
        }
        JavaNodePayload::EnumConstant(e) => {
            let _ = write!(
                out,
                "```java\n{}\n```\n\nenum constant `{}`",
                e.header.name, e.header.fqn
            );
        }
        JavaNodePayload::Type(t) => {
            let tp = if t.type_parameters.is_empty() {
                String::new()
            } else {
                let names: Vec<&str> = t.type_parameters.iter().map(|p| p.name.as_str()).collect();
                format!("<{}>", names.join(", "))
            };
            let kind_word = match t.kind {
                JavaTypeKind::Class => "class",
                JavaTypeKind::Interface => "interface",
                JavaTypeKind::Enum => "enum",
                JavaTypeKind::Record => "record",
                JavaTypeKind::Annotation => "@interface",
            };
            let _ = write!(
                out,
                "```java\n{} {}{}\n```\n\n`{}`",
                kind_word, t.header.name, tp, t.header.fqn
            );
        }
        JavaNodePayload::AnnotationElement(e) => {
            let _ = write!(
                out,
                "```java\n{} {}()\n```\n\n`{}`",
                e.element_type, e.header.name, e.header.fqn
            );
        }
        JavaNodePayload::Package(p) => {
            let _ = write!(out, "```java\npackage {}\n```", p.header.fqn);
        }
        JavaNodePayload::Parameter(_) => return None,
        // Use-site nodes are not hover targets; resolution at a cursor
        // lands on the declaration the use site refers to, not on the
        // use site itself.
        JavaNodePayload::TypeUse(_) => return None,
        JavaNodePayload::Import(_) => return None,
    }

    Some(out)
}
