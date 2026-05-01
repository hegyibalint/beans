//! Cross-registry query abstraction.
//!
//! [`Queryable<M>`] is the trait registries implement once per query
//! shape they answer. The native case is trivial: each [`Registry<K>`]
//! impls `Queryable<K>` returning a [`QueryResult`] by calling its own
//! `providers`. Cross-registry query *models* (e.g. [`ByFqn`]) are
//! separate types, with one `Queryable<M>` impl per registry that
//! answers the model — translating the model into the registry's
//! native key on the way through.
//!
//! [`QueryResult`] is a tri-state owned value: `None`, `One(NodeId)`, or
//! `Many(Vec<NodeId>)`. The variant tells the consumer the cardinality
//! at a glance; the zero/one cases never allocate a Vec; pattern matches
//! at the call site make the cardinality explicit. Per ADR-0007 the
//! NodeIds inside are generational handles — safe to hold across
//! registry mutations and to dereference later via the graph's
//! generation-validated `get` (returns `None` if the slot was
//! destroyed).
//!
//! Cross-registry consumption uses [`first_match`] / [`all_matches`]
//! over a `&[&dyn Queryable<M>]` of the registries to consult, named
//! by the caller (no hidden routing). This is the priority-list
//! pattern from ADR-0008 — when language-native rich models start
//! diverging from JVM projection, a Java-side caller might consult
//! `[&beans.registries.java_symbols, &beans.registries.jvm_types]` so
//! the language-native answer wins when present.

use crate::graph::NodeId;
use crate::jvm::Fqn;
use crate::registry::Registry;
use std::hash::Hash;

/// Tri-state owned query result. Owns the matching `NodeId`s; no
/// borrows escape, so the caller can pattern-match, store, send across
/// channels, or use `?`-chain through `Beans::get` for dereferencing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryResult {
    None,
    One(NodeId),
    Many(Vec<NodeId>),
}

impl QueryResult {
    /// `Some(id)` if there is at least one match (`One` or first of
    /// `Many`), `None` otherwise. Convenience for go-to-definition style
    /// callers that want a single representative.
    pub fn first(&self) -> Option<NodeId> {
        match self {
            QueryResult::None => None,
            QueryResult::One(id) => Some(*id),
            QueryResult::Many(ids) => ids.first().copied(),
        }
    }

    /// Consume into an owned `Vec<NodeId>`. Empty for `None`, length-1
    /// for `One`, the original Vec for `Many`. Convenience for callers
    /// that want a uniform iteration shape.
    pub fn all(self) -> Vec<NodeId> {
        match self {
            QueryResult::None => Vec::new(),
            QueryResult::One(id) => vec![id],
            QueryResult::Many(ids) => ids,
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, QueryResult::None)
    }

    /// Number of matching providers.
    pub fn count(&self) -> usize {
        match self {
            QueryResult::None => 0,
            QueryResult::One(_) => 1,
            QueryResult::Many(ids) => ids.len(),
        }
    }
}

impl From<Vec<NodeId>> for QueryResult {
    fn from(v: Vec<NodeId>) -> Self {
        match v.len() {
            0 => QueryResult::None,
            1 => QueryResult::One(v[0]),
            _ => QueryResult::Many(v),
        }
    }
}

/// A registry that can answer a query of shape `M`.
///
/// Each [`Registry<K>`] impls `Queryable<K>` for its own key (the native
/// case). Cross-registry models like [`ByFqn`] are answered by every
/// registry that can translate the model into its native key — so a
/// caller can hand a `&[&dyn Queryable<ByFqn>]` to [`first_match`] and
/// the right registries answer with no caller-side dispatch.
pub trait Queryable<M> {
    fn query(&self, model: &M) -> QueryResult;
}

/// Native-key impl: every `Registry<K>` answers queries of its own key.
impl<K: Eq + Hash + Clone> Queryable<K> for Registry<K> {
    fn query(&self, key: &K) -> QueryResult {
        QueryResult::from(self.providers(key))
    }
}

/// Cross-registry "find me anything with this FQN" query model.
/// Multiple registries answer this; each translates the FQN into its
/// own native key.
#[derive(Debug, Clone)]
pub struct ByFqn(pub Fqn);

impl Queryable<ByFqn> for Registry<crate::jvm::JvmTypeKey> {
    fn query(&self, m: &ByFqn) -> QueryResult {
        QueryResult::from(self.providers(&crate::jvm::JvmTypeKey::new(m.0.clone())))
    }
}

#[cfg(feature = "java")]
impl Queryable<ByFqn> for Registry<crate::languages::java::JavaSymbolKey> {
    fn query(&self, m: &ByFqn) -> QueryResult {
        QueryResult::from(
            self.providers(&crate::languages::java::JavaSymbolKey::new(m.0.clone())),
        )
    }
}

/// First-match across the given registries, in priority order. The
/// caller names which registries to consult and in what order. Returns
/// the first non-empty answer; subsequent registries are not consulted.
///
/// This is the resolution-style entry point (go-to-definition,
/// type-checking). Callers that want every match across every consulted
/// registry use [`all_matches`].
pub fn first_match<M>(model: &M, consult: &[&dyn Queryable<M>]) -> Option<NodeId> {
    consult.iter().find_map(|r| r.query(model).first())
}

/// Every match across the given registries, in query order, no dedup.
/// Per ADR-0008 the cross-registry merge does not de-duplicate — a
/// node registered under multiple keys appears once per hit. Consumers
/// that want one entry per logical symbol (typical for completion)
/// collapse downstream with knowledge of which language wins.
pub fn all_matches<M>(model: &M, consult: &[&dyn Queryable<M>]) -> Vec<NodeId> {
    consult
        .iter()
        .flat_map(|r| r.query(model).all())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jvm::JvmTypeKey;

    #[test]
    fn query_result_from_vec() {
        let r: QueryResult = Vec::<NodeId>::new().into();
        assert!(matches!(r, QueryResult::None));

        let r: QueryResult = vec![NodeId::placeholder(1)].into();
        assert!(matches!(r, QueryResult::One(_)));

        let r: QueryResult = vec![NodeId::placeholder(1), NodeId::placeholder(2)].into();
        assert!(matches!(r, QueryResult::Many(_)));
    }

    #[test]
    fn query_result_count_and_first() {
        let r = QueryResult::None;
        assert_eq!(r.count(), 0);
        assert_eq!(r.first(), None);

        let r = QueryResult::One(NodeId::placeholder(7));
        assert_eq!(r.count(), 1);
        assert_eq!(r.first(), Some(NodeId::placeholder(7)));

        let r = QueryResult::Many(vec![NodeId::placeholder(7), NodeId::placeholder(8)]);
        assert_eq!(r.count(), 2);
        assert_eq!(r.first(), Some(NodeId::placeholder(7)));
    }

    #[test]
    fn registry_native_queryable() {
        let r: Registry<JvmTypeKey> = Registry::new();
        let key = JvmTypeKey::new("com.example.Foo");

        // Empty: query returns None.
        assert!(matches!(r.query(&key), QueryResult::None));

        let _h = r.register(key.clone(), NodeId::placeholder(1));
        assert!(matches!(r.query(&key), QueryResult::One(_)));

        let _h2 = r.register(key.clone(), NodeId::placeholder(2));
        assert!(matches!(r.query(&key), QueryResult::Many(_)));
    }

    #[test]
    fn first_match_walks_in_priority_order() {
        let r1: Registry<JvmTypeKey> = Registry::new();
        let r2: Registry<JvmTypeKey> = Registry::new();
        let key = JvmTypeKey::new("com.example.Foo");

        let _h2 = r2.register(key.clone(), NodeId::placeholder(99));

        // Only r2 has a provider; first_match falls through.
        let result = first_match::<JvmTypeKey>(&key, &[&r1, &r2]);
        assert_eq!(result, Some(NodeId::placeholder(99)));

        // r1 gets a higher-priority provider.
        let _h1 = r1.register(key.clone(), NodeId::placeholder(7));
        let result = first_match::<JvmTypeKey>(&key, &[&r1, &r2]);
        assert_eq!(result, Some(NodeId::placeholder(7)));
    }

    #[test]
    fn all_matches_concatenates_query_order() {
        let r1: Registry<JvmTypeKey> = Registry::new();
        let r2: Registry<JvmTypeKey> = Registry::new();
        let key = JvmTypeKey::new("com.example.Foo");

        let _h1 = r1.register(key.clone(), NodeId::placeholder(1));
        let _h2 = r2.register(key.clone(), NodeId::placeholder(2));

        let results = all_matches::<JvmTypeKey>(&key, &[&r1, &r2]);
        assert_eq!(
            results,
            vec![NodeId::placeholder(1), NodeId::placeholder(2)]
        );
    }
}
