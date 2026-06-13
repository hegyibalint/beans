//! Fully-qualified names.
//!
//! Per ADR-0007 the engine's [`NodeId`](beans_core::graph::NodeId) is a runtime
//! arena index, not a stable identity. Semantic identity lives in registry
//! keys, and the keys lean on fully-qualified names. [`Fqn`] is the
//! single source-of-truth wrapper for those — defined once here so that
//! `JvmTypeKey`, `JvmMethodKey`, and the per-language equivalents share a
//! comparable representation.
//!
//! For now [`Fqn`] is a thin newtype around [`String`]. ADR-0008 calls out
//! that link objects exist at the scale of millions in a real project and
//! that "we need to keep the query objects compact"; when that becomes
//! load-bearing we will swap the inner storage for `Arc<str>` or a string
//! intern table without changing the public API.

use std::fmt;
use std::sync::Arc;

use beans_core::Interner;

/// A dotted, fully-qualified name as it appears in cross-file lookups:
/// `com.example.Service`, `java.util.List`, `com.example.Service.process`.
///
/// Backed by `Arc<str>` (backlog #037): equality and hashing are
/// content-based, so interned and uninterned values mix freely —
/// cloning is a pointer bump either way, and [`Fqn::intern_in`]
/// collapses identical text onto one buffer at the integrate boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fqn(Arc<str>);

impl Fqn {
    /// Construct an [`Fqn`] from any owned-or-borrowed string.
    pub fn new(name: impl Into<String>) -> Self {
        Fqn(Arc::from(name.into()))
    }

    /// Re-key onto the workspace's canonical buffer for this text.
    pub fn intern_in(&mut self, interner: &Interner) {
        self.0 = interner.intern(&self.0);
    }

    /// Borrow the dotted form.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Copy out as an owned [`String`].
    pub fn into_string(self) -> String {
        self.0.as_ref().to_string()
    }

    /// The last segment of the dotted form: `"com.example.Service"` →
    /// `"Service"`. The whole name when there is no dot.
    pub fn simple_name(&self) -> &str {
        match self.0.rfind('.') {
            Some(dot) => &self.0[dot + 1..],
            None => &self.0,
        }
    }

    /// Split off the last segment: `"com.example.Service.process"` →
    /// `("com.example.Service", "process")`. Returns `None` for an
    /// unqualified name (no dot).
    pub fn split_last(&self) -> Option<(Fqn, &str)> {
        let dot = self.0.rfind('.')?;
        let parent = &self.0[..dot];
        let leaf = &self.0[dot + 1..];
        Some((Fqn::new(parent), leaf))
    }
}

impl fmt::Display for Fqn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for Fqn {
    fn from(value: &str) -> Self {
        Fqn::new(value)
    }
}

impl From<String> for Fqn {
    fn from(value: String) -> Self {
        Fqn(Arc::from(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_last_qualified() {
        let fqn = Fqn::new("com.example.Service.process");
        let (parent, leaf) = fqn.split_last().unwrap();
        assert_eq!(parent.as_str(), "com.example.Service");
        assert_eq!(leaf, "process");
    }

    #[test]
    fn split_last_unqualified() {
        let fqn = Fqn::new("Service");
        assert!(fqn.split_last().is_none());
    }

    #[test]
    fn display_round_trips() {
        let fqn = Fqn::new("a.b.c");
        assert_eq!(fqn.to_string(), "a.b.c");
        assert_eq!(fqn.as_str(), "a.b.c");
    }
}
