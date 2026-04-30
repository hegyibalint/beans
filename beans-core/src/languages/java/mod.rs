//! Java language module.
//!
//! Per ADR-0004 / ADR-0019 each JVM language has its own module under
//! [`crate::languages`]. Java's source model maps closely to its JVM
//! projection, so this module is mostly a thin Java-specific overlay on
//! top of [`crate::jvm`]:
//!
//! - [`keys`] / [`registries`] — the single Java-side key (`JavaSymbolKey`)
//!   and registry (`JavaRegistries`).
//! - [`payload`] — the typed `JavaNodePayload` variants.
//! - [`parser`] — the tree-sitter walker; per ADR-0021 the walker
//!   structure is preserved verbatim while the layers above it are
//!   rewritten.
//! - [`types`] — Java-local `TypeRef` shape used by the walker; converts
//!   to the canonical [`crate::TypeRef`] via `to_core`.
//! - [`syntax`] — language-local `extract_imports`, `extract_package`,
//!   `word_at_position` helpers; consumed by the fixture harness and the
//!   LSP via direct calls (no `Language` trait per ADR-0021).
//! - [`resolve`] — in-file name resolution chain (FQN → imports →
//!   same-package → wildcard → static → simple-name fallback). Returns
//!   `NodeId` so consumers can format their own result types.

pub mod keys;
pub mod parser;
pub mod payload;
pub mod registries;
pub mod resolve;
pub mod syntax;
pub mod types;

pub use keys::JavaSymbolKey;
pub use parser::{integrate, parse_java_to_graph, ParsedJavaFile};
pub use payload::{
    JavaAnnotationElementNode, JavaConstructorNode, JavaDeclHeader, JavaEnumConstantNode,
    JavaFieldNode, JavaMethodNode, JavaNodePayload, JavaPackageNode, JavaParameter,
    JavaTypeKind, JavaTypeNode,
};
pub use registries::JavaRegistries;
pub use resolve::{lookup_fqn, resolve_compound_name, resolve_name, resolve_simple_name};
pub use syntax::{extract_imports, extract_package, word_at_position, Import};
