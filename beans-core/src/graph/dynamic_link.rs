//! Cross-registry queries.
//!
//! ADR-0006 distinguishes hard links (intra-file ownership, stored as
//! `Vec<NodeId>` on `NodeData::children`) from "dynamic links" — cross-file
//! lookups that must survive their target's deletion and recreation. The
//! load-bearing piece for the second is the question itself: what does the
//! use-site store so that lookup remains valid as files come and go?
//!
//! ADR-0008 answers: the use-site stores **the question, not the answer**.
//! The question is some [`RegistryQuery`] impl that knows how to consult
//! one or more typed registries against the current
//! [`Registries`](crate::Registries) state.
//! At lookup time the query produces fresh [`NodeId`]s; nothing dangles
//! across edits because the use-site never cached a target id in the
//! first place.
//!
//! Today this module is just the trait. ADR-0008 also describes a
//! "priority list of queries with FirstMatch / MergeAll combine modes
//! and tiered subscriptions" — a richer abstraction that earns its weight
//! when (a) per-language node payloads carry information the JVM
//! projection loses (so language-native first / JVM fallback is a real
//! choice) and (b) cached cross-file resolutions exist that can go stale.
//! Neither holds today, so the larger abstraction is deferred. When it
//! lands it should be named for what it is — `MultiQuery<Q>` or similar
//! — and decide its own caching/subscription policy at that point.
//!
//! Use-sites that need the simple priority-then-fallback shape today can
//! inline it as an iterator chain over their [`RegistryQuery`] impls:
//!
//! ```text
//! queries.iter().flat_map(|q| q.resolve(ctx)).next()       // FirstMatch
//! queries.iter().flat_map(|q| q.resolve(ctx)).collect()    // MergeAll
//! ```

use crate::graph::arena::NodeId;

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
///             JavaQuery::Java(k) => ctx.java_symbols.providers(k),
///             JavaQuery::Jvm(k)  => ctx.jvm_types.providers(k),
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
