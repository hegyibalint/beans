//! The flat-record symbol type used by the prototype symbol model.
//!
//! `Symbol` is the universal record for every declared thing in a project —
//! classes, methods, fields, packages, parameters. The prototype's design
//! is to carry every possibly-relevant piece of information in one struct
//! and pattern-match on `kind` + `signature` at consumer sites.
//!
//! Per ADR-0021 the graph engine replaces this with typed payload
//! variants, so consumers no longer pattern-match on optionals. `Symbol`
//! and its surrounding `SymbolTable` are disposable per ADR-0003 — the
//! abstractions live on as the *shape* of node payloads, but the
//! one-struct-fits-all design does not.

use crate::{AnnotationInstance, Location, Modifier, Relation, Signature, SymbolId, SymbolKind};

/// A declared element in the project: class, method, field, package, etc.
///
/// Every declaration the indexer encounters becomes one `Symbol` in the
/// owning [`SymbolTable`](crate::SymbolTable). The table arena owns the
/// symbols; everything else holds [`SymbolId`]s into it.
///
/// The design is intentionally flat: a single struct with optional fields
/// rather than a typed enum of variants. The cost is that consumers must
/// pattern-match `kind` to know which fields are meaningful (a `Package`
/// has no `signature`; a `Method` does). The reward, in the prototype,
/// is uniform handling — one collection, one filter, one query API. The
/// graph-engine replacement gives that up in exchange for stronger types.
///
/// ## Field semantics
///
/// - `id`, `parent`, `children` form the containment tree. A class
///   contains methods and fields; a method contains parameters; a package
///   contains classes. The root symbols (top-level packages) have
///   `parent: None`.
/// - `relations` carries typed cross-tree edges (extends/implements/
///   overrides). These can cross containment boundaries; `children`
///   cannot.
/// - `signature` carries kind-specific structural data when applicable.
///   Inspect `kind` first to know which `Signature` variant to expect.
///
/// ## Identity
///
/// Two symbols compare equal iff every field compares equal. This is
/// useful for tests but expensive at runtime; production lookups go
/// through [`SymbolId`] or `fqn`, not through `PartialEq`.
#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    /// Handle into the owning symbol table. Stable for the lifetime of
    /// that table; not stable across rebuilds (per the same rules as
    /// the graph's `NodeId`, ADR-0007).
    pub id: SymbolId,
    /// Fully qualified name. The canonical durable identifier — survives
    /// rebuilds and is the key callers use when they need to refer to
    /// this symbol by name (cross-file resolution, snapshot diffs).
    /// Format is dotted (`com.example.Service.process`); the structure
    /// after the package depends on `kind`.
    pub fqn: String,
    /// Simple (unqualified) name as written. For a method it is just the
    /// method name without parentheses; for a class, the class name
    /// without the package prefix.
    pub name: String,
    /// What the symbol declares — class, method, field, etc. Drives
    /// nearly every downstream pattern match.
    pub kind: SymbolKind,
    /// Source range of the declaration. `None` for symbols loaded from
    /// compiled artifacts (jmod, JAR) or for synthetic symbols (e.g.,
    /// the implicit package for a default-package class).
    pub location: Option<Location>,
    /// Source-level modifiers (`public`, `static`, `final`, ...) in the
    /// order written in source. Empty for declarations with no
    /// modifiers and for symbols that cannot carry modifiers
    /// (parameters, packages).
    pub modifiers: Vec<Modifier>,
    /// Annotations applied to this declaration.
    pub annotations: Vec<AnnotationInstance>,
    /// Containing symbol — package for top-level types, type for
    /// members, method for parameters. `None` only for top-level
    /// packages.
    pub parent: Option<SymbolId>,
    /// Symbols this one contains. The parent/children relationship
    /// matches source nesting: a class's `children` are its members;
    /// a package's `children` are its top-level types and sub-packages.
    pub children: Vec<SymbolId>,
    /// Typed edges to other symbols: extends, implements, overrides,
    /// permits. Distinct from `children` because relations express
    /// non-containment links and may cross packages or files.
    pub relations: Vec<Relation>,
    /// Kind-specific structural data. Always `None` for kinds that
    /// have nothing to add (e.g., [`Package`](crate::SymbolKind::Package),
    /// [`Parameter`](crate::SymbolKind::Parameter)). Inspect `kind`
    /// to know which [`Signature`] variant to expect; mismatched
    /// pairings are bugs in the populator.
    pub signature: Option<Signature>,
}
