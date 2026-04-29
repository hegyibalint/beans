//! Clojure-specific symbol kinds.

/// A Clojure-specific kind. Variants here describe constructs the JVM
/// projection cannot represent without losing the information Clojure
/// consumers need.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    /// Clojure `(ns my.thing ...)` declaration. The closest analogue to a
    /// Java package, but Clojure namespaces also own `def`s directly.
    Namespace,
    /// Clojure `(defn name ...)` — top-level function bound in a namespace.
    /// JVM-projected to a static method on the namespace's compiled class.
    Function,
    /// Clojure `(defprotocol P ...)` — open polymorphic dispatch over a
    /// type. JVM-projected to an interface plus a dispatch table.
    Protocol,
    /// Clojure `(defmulti name dispatch-fn)` — multi-method with arbitrary
    /// dispatch. No direct JVM construct; modelled separately.
    Multimethod,
    /// Clojure `(defrecord Name [...])` — generates a Java class with
    /// typed fields, plus protocol implementations.
    Defrecord,
    /// Clojure `(deftype Name [...])` — like [`Self::Defrecord`] but
    /// without the map-like behaviour layered on.
    Deftype,
}
