//! beans-lang-java — the Java vertical.
//!
//! Per ADR-0004 each JVM language owns its rich model in its own
//! crate, depending only on the engine (`beans-core`) and the shared
//! JVM model (`beans-lang-jvm`) — never on another language. Java's
//! source model maps closely to its JVM projection, so this crate is
//! mostly a thin Java-specific overlay on top of `beans-lang-jvm`:
//!
//! - [`keys`] — the single Java-side key (`JavaSymbolKey`).
//! - [`registries`] — the Java registry bag ([`JavaRegistries`]); the
//!   `beans` facade composes it with [`beans_lang_jvm::JvmRegistries`].
//! - [`payload`] — the typed `JavaNodePayload` variants plus the
//!   [`AsJava`](payload::AsJava) projection trait generic vertical code
//!   is written against.
//! - [`parser`] — the tree-sitter walker; per ADR-0021 the walker
//!   structure is preserved verbatim while the layers above it are
//!   rewritten. Generic over the graph payload `P` via `From` bounds.
//! - [`types`] — Java-local `TypeRef` shape used by the walker;
//!   converts to the canonical `beans_lang_jvm::TypeRef` via `to_core`.
//! - [`syntax`] — language-local `extract_imports`, `extract_package`,
//!   `word_at_position` helpers.
//! - [`resolve`] — in-file name resolution chain (FQN → imports →
//!   same-package → wildcard → static → simple-name fallback).
//! - [`diagnostics`] — the Java rule set, dispatched by the facade's
//!   `compute_diagnostics`.

pub mod diagnostics;
pub mod fixes;
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
    AsJava, JavaAnnotationElementNode, JavaConstructorNode, JavaDeclHeader, JavaEnumConstantNode,
    JavaFieldNode, JavaImportNode, JavaImportKind, JavaMethodNode, JavaNodePayload,
    JavaPackageNode, JavaParameter, JavaTypeKind, JavaTypeNode, JavaTypeUseNode, JavaUseHeader,
};
pub use fixes::{add_import_fix, import_candidates, quick_fixes_at, type_use_at};
pub use registries::JavaRegistries;
pub use resolve::{lookup_fqn, resolve_compound_name, resolve_name, resolve_simple_name};
pub use syntax::{extract_imports, extract_package, word_at_position, Import};
