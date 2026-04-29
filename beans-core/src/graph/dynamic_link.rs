//! Dynamic links and registry queries.
//!
//! Per ADR-0006 dynamic links are cross-file dependency edges, mediated by
//! registries. A use site referencing `Service.process` does not store a
//! target [`NodeId`] — it stores an ordered list of registry queries plus a
//! cached result for whichever query is currently active.
//!
//! Per ADR-0008 each link carries:
//! - An ordered list of queries (highest priority first).
//! - A combine mode ([`LinkMode::FirstMatch`] for resolution,
//!   [`LinkMode::MergeAll`] for completion).
//! - An active query index and a cached result. When the active query
//!   stops returning a hit, the link falls through to the next query in
//!   the list and re-caches.
//!
//! Per ADR-0012 each registry has its own typed key. The link does not
//! commit to a single key type; instead it is generic over a query enum
//! `Q` that the consumer defines (typically `enum JavaQuery { Java(...),
//! Jvm(...), ... }`) and that implements [`RegistryQuery`]. The trait's
//! associated [`Ctx`](RegistryQuery::Ctx) is the registry bag the consumer
//! exposes (typically `Registries`); the engine never names specific
//! registries.
//!
//! What this module deliberately does *not* implement yet:
//! - Tiered subscriptions (value-watch on the active query, existence-
//!   watch on higher-priority queries). See ADR-0008's "subscriptions
//!   tiered by query position." That is non-trivial state-machine work
//!   and is not on the critical path for the migration's steps 3-8;
//!   resolution and fallback are. When subscriptions land, they wire in
//!   alongside [`DynamicLink`] without changing its public API.
//! - MergeAll-time dedup. Per ADR-0008 dedup is "language-specific"
//!   (Kotlin-defined symbols win over JVM projections of themselves).
//!   The dedup rule lives in the consumer that knows the languages
//!   involved; the link returns the unioned `Vec<NodeId>` in query
//!   order and lets the consumer collapse duplicates.

use crate::graph::arena::NodeId;

/// How a [`DynamicLink`] combines results from its query list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkMode {
    /// Try queries in order; the first non-empty result wins. Used for
    /// go-to-definition, type-checking, and any operation that needs a
    /// single authoritative target.
    FirstMatch,
    /// Run every query and union all results in query order. Used for
    /// completion, which surfaces every plausible candidate.
    MergeAll,
}

/// A registry query the consumer can resolve against a context of
/// registries. Implementations are typically thin enum dispatch:
///
/// ```text
/// enum JavaQuery {
///     Java(JavaSymbolKey),
///     Jvm(JvmTypeKey),
/// }
/// impl RegistryQuery for JavaQuery {
///     type Ctx = Registries;
///     fn resolve(&self, ctx: &Self::Ctx) -> Vec<NodeId> {
///         match self {
///             JavaQuery::Java(k) => ctx.java.query(k),
///             JavaQuery::Jvm(k)  => ctx.jvm_types.query(k),
///         }
///     }
/// }
/// ```
///
/// Per ADR-0012 the typed-key discipline lives at the *registry*. The
/// query enum exists only to homogenise heterogeneous registries behind
/// one trait at the use site.
pub trait RegistryQuery {
    /// The bag-of-registries the consumer threads through resolution.
    type Ctx;

    /// Run this query against the consumer's registries and return every
    /// matching node. Order within the returned `Vec` is the registry's
    /// own provider order (per ADR-0013 it carries no semantic weight at
    /// the registry layer).
    fn resolve(&self, ctx: &Self::Ctx) -> Vec<NodeId>;
}

/// A dynamic link from one node to a target identified by registry
/// queries.
///
/// The link is *not* registered anywhere on its own — it is a value held
/// by the source node, typically inside a `Vec<DynamicLink<...>>` on the
/// node's payload. Registry providers are registered separately on the
/// target node; resolution walks the link's queries against the registry
/// to find the current target.
#[derive(Debug, Clone)]
pub struct DynamicLink<Q> {
    queries: Vec<Q>,
    mode: LinkMode,
    active_index: Option<usize>,
    cached_result: Option<NodeId>,
}

impl<Q> DynamicLink<Q> {
    /// Create a link with an explicit mode.
    pub fn new(queries: Vec<Q>, mode: LinkMode) -> Self {
        Self {
            queries,
            mode,
            active_index: None,
            cached_result: None,
        }
    }

    /// Convenience constructor for a [`LinkMode::FirstMatch`] link.
    pub fn first_match(queries: Vec<Q>) -> Self {
        Self::new(queries, LinkMode::FirstMatch)
    }

    /// Convenience constructor for a [`LinkMode::MergeAll`] link.
    pub fn merge_all(queries: Vec<Q>) -> Self {
        Self::new(queries, LinkMode::MergeAll)
    }

    /// The queries this link will consult, in priority order.
    pub fn queries(&self) -> &[Q] {
        &self.queries
    }

    /// The link's combine mode.
    pub fn mode(&self) -> LinkMode {
        self.mode
    }

    /// Index into [`queries`](Self::queries) whose result is currently
    /// cached, if any. Only meaningful for [`LinkMode::FirstMatch`]; for
    /// [`LinkMode::MergeAll`] this stays `None` because no single query
    /// is "active."
    pub fn active_index(&self) -> Option<usize> {
        self.active_index
    }

    /// Cached single-result for [`LinkMode::FirstMatch`] links. `None`
    /// before [`resolve`](Self::resolve) is called, and `None` if no
    /// query in the list returned a hit on the last call. Always `None`
    /// for [`LinkMode::MergeAll`] links.
    pub fn cached_result(&self) -> Option<NodeId> {
        self.cached_result
    }

    /// Drop any cached active index and result. Call this when the
    /// underlying registries change in a way that may have invalidated
    /// the cached value (the next [`resolve`](Self::resolve) call will
    /// re-walk the query list).
    pub fn invalidate(&mut self) {
        self.active_index = None;
        self.cached_result = None;
    }
}

impl<Q: RegistryQuery> DynamicLink<Q> {
    /// Resolve this link to a single target.
    ///
    /// For [`LinkMode::FirstMatch`] (the standard mode): walk the query
    /// list in order; the first query whose [`resolve`](RegistryQuery::resolve)
    /// returns at least one [`NodeId`] wins. The link caches the
    /// `(active_index, cached_result)` pair so subsequent calls without
    /// an [`invalidate`](Self::invalidate) return the cached value
    /// directly.
    ///
    /// For [`LinkMode::MergeAll`] this still picks the first hit (the
    /// merge-all results don't compose into a single answer); use
    /// [`resolve_all`](Self::resolve_all) when the caller wants every
    /// candidate.
    ///
    /// When a query returns multiple providers, this function picks the
    /// first one. Per ADR-0013 the registry's provider order carries no
    /// semantic weight, so callers that need precedence among multiple
    /// hits in a single registry must encode that as additional queries
    /// or apply a resolution rule outside the link.
    pub fn resolve(&mut self, ctx: &Q::Ctx) -> Option<NodeId> {
        for (idx, query) in self.queries.iter().enumerate() {
            let hits = query.resolve(ctx);
            if let Some(&first) = hits.first() {
                self.active_index = Some(idx);
                self.cached_result = Some(first);
                return Some(first);
            }
        }
        self.active_index = None;
        self.cached_result = None;
        None
    }

    /// Resolve this link to every candidate target.
    ///
    /// Walks every query in the list (regardless of [`mode`](Self::mode))
    /// and returns every provider in query order. Per ADR-0008 this is
    /// the operation completion uses; per the same ADR the consumer is
    /// responsible for any language-specific dedup (e.g., Kotlin-defined
    /// symbols winning over JVM projections of themselves).
    ///
    /// Does not cache — completion candidates change too often for a
    /// single-NodeId cache to mean anything. Callers that want to memoise
    /// the result do so externally.
    pub fn resolve_all(&self, ctx: &Q::Ctx) -> Vec<NodeId> {
        let mut out = Vec::new();
        for query in &self.queries {
            out.extend(query.resolve(ctx));
        }
        out
    }
}
