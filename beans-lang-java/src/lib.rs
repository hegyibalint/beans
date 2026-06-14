//! beans-lang-java — the Java vertical.
//!
//! Per ADR-0004 each JVM language owns its rich model in its own
//! crate, depending only on the engine (`beans-core`) and the shared
//! JVM model (`beans-lang-jvm`) — never on another language. Java's
//! source model maps closely to its JVM projection, so this crate is
//! mostly a thin Java-specific overlay on top of `beans-lang-jvm`:
//!
//! - [`model`] — Java-side keys, payloads, registries, and parser-local
//!   type references.
//! - [`parse`] — the tree-sitter walker; per ADR-0021 the walker
//!   structure is preserved verbatim while the layers above it are
//!   rewritten. Generic over the graph payload `P` via `From` bounds.
//! - [`source`] — language-local `extract_imports`, `extract_package`,
//!   `word_at_position` helpers.
//! - [`resolve`] — in-file name resolution chain (FQN → imports →
//!   same-package → wildcard → static → simple-name fallback).
//! - [`diagnostics`] — the Java rule set, dispatched by the facade's
//!   `compute_diagnostics`.

pub mod diagnostics;
pub mod fixes;
pub mod model;
pub mod parse;
pub mod resolve;
pub mod source;

pub use fixes::{add_import_fix, import_candidates, quick_fixes_at, type_use_at};
pub use model::JavaRegistries;
pub use model::{
    AsJava, JavaAnnotationElementNode, JavaConstructorNode, JavaDeclHeader, JavaEnumConstantNode,
    JavaFieldNode, JavaImportKind, JavaImportNode, JavaMethodNode, JavaNodePayload,
    JavaPackageNode, JavaParameter, JavaSymbolKey, JavaTypeKind, JavaTypeNode, JavaTypeUseNode,
    JavaUseHeader,
};
pub use parse::{ParsedJavaFile, parse_java_to_graph};
pub use resolve::{lookup_fqn, resolve_compound_name, resolve_name, resolve_simple_name};
pub use source::{
    Import, compound_at_position, extract_imports, extract_package, word_at_position,
};

// Compatibility module aliases. Keep these until the facade/API cleanup
// removes broad root module paths.
pub use model::keys;
pub use model::payload;
pub use model::registries;
pub use model::type_ref as types;
pub use parse as parser;
pub use source as syntax;
