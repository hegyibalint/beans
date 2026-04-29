//! Java-side registry keys.
//!
//! Per ADR-0012 each registry has its own typed key. Java has fewer
//! distinct lookup shapes than the JVM projection because the Java
//! model maps almost cleanly to JVM kinds — there is one `JavaSymbolKey`
//! that addresses any Java-side declaration by FQN. Method overloads
//! resolve through the JVM projection (the per-method JVM node is
//! hard-linked off its Java parent), so the Java-side key does not need
//! a parameter list.

use crate::jvm::fqn::Fqn;

/// Key identifying a Java-side declaration by its fully-qualified name.
/// One key per Java type, method-overload group, field, or package.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JavaSymbolKey {
    pub fqn: Fqn,
}

impl JavaSymbolKey {
    pub fn new(fqn: impl Into<Fqn>) -> Self {
        Self { fqn: fqn.into() }
    }
}
