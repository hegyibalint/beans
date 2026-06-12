//! Java in-file name resolution.
//!
//! Per ADR-0012 the registries are FQN-keyed; per JLS §6.4 a Java
//! identifier at a use site is interpreted by walking a fixed chain of
//! qualifications (the same identifier can mean different things in
//! different files). This module hosts that chain so every consumer
//! (the LSP, the fixture harness) talks to the same one.
//!
//! The chain (matches the prototype's `resolve::resolve_name`):
//!
//! 1. **Exact FQN** — the bare name *is* a fully-qualified name.
//! 2. **Explicit imports** — `import com.example.MyClass;` makes
//!    `MyClass` mean `com.example.MyClass` (JLS §7.5.1).
//! 3. **Same-package qualification** — `MyClass` in a file declared in
//!    package `p` resolves to `p.MyClass` (JLS §7.4.1).
//! 4. **Wildcard imports** — `import java.util.*;` makes `ArrayList`
//!    mean `java.util.ArrayList` (JLS §7.5.2).
//! 5. **Static imports** — `import static com.example.Utils.MAX;`
//!    makes `MAX` mean `com.example.Utils.MAX` (JLS §7.5.3 / §7.5.4).
//! 6. **Simple-name fallback** — last-resort iteration over the graph
//!    for a uniquely-named declaration. Prototype semantics; expensive
//!    at scale and intended only for the fallback case.
//!
//! The function returns a [`NodeId`]; LSP-shaped output (Location,
//! Hover, etc.) is the consumer's job — per ADR-0020 LSP types do not
//! enter this layer.

use beans_core::graph::arena::{Graph, NodeId};
use beans_lang_jvm::fqn::Fqn;
use beans_lang_jvm::keys::{JvmTypeKey, PackageKey};
use beans_lang_jvm::registries::JvmRegistries;
use crate::keys::JavaSymbolKey;
use crate::payload::AsJava;
use crate::registries::JavaRegistries;
use crate::syntax::Import;

/// Resolve a Java identifier at a use site.
///
/// `imports` and `current_package` are file-local context; `name` is the
/// bare identifier (or compound name like `"MyClass.doWork"` — the
/// chain treats it as one string and lets the FQN exact-match step
/// catch compound forms when both pieces resolve).
///
/// Per ADR-0012 every step but the simple-name fallback queries the
/// registries; the fallback walks the graph because the registries are
/// FQN-keyed and have no name-only entry point. The expense of the
/// fallback is acceptable because it only fires when the preceding
/// five steps have already missed.
pub fn resolve_name<P: AsJava>(
    name: &str,
    imports: &[Import],
    current_package: &str,
    java: &JavaRegistries,
    jvm: &JvmRegistries,
    graph: &Graph<P>,
) -> Option<NodeId> {
    // 1. Exact FQN.
    if let Some(id) = lookup_fqn(java, jvm, name) {
        return Some(id);
    }

    // 2. Explicit imports.
    for import in imports {
        if let Import::Single(fqn, _) = import
            && (fqn.ends_with(&format!(".{}", name)) || fqn == name)
            && let Some(id) = lookup_fqn(java, jvm, fqn)
        {
            return Some(id);
        }
    }

    // 3. Same-package qualification.
    if !current_package.is_empty() {
        let candidate = format!("{}.{}", current_package, name);
        if let Some(id) = lookup_fqn(java, jvm, &candidate) {
            return Some(id);
        }
    }

    // 4. Wildcard imports.
    for import in imports {
        if let Import::Wildcard(package, _) = import {
            let candidate = format!("{}.{}", package, name);
            if let Some(id) = lookup_fqn(java, jvm, &candidate) {
                return Some(id);
            }
        }
    }

    // 5. Static imports.
    for import in imports {
        if let Import::Static(fqn, _) = import
            && (fqn.ends_with(&format!(".{}", name)) || fqn == name)
            && let Some(id) = lookup_fqn(java, jvm, fqn)
        {
            return Some(id);
        }
    }

    // 6. Simple-name fallback.
    resolve_simple_name(name, graph)
}

/// Resolve a compound name like `Type.method` or `Type.field` against
/// the same chain as [`resolve_name`]: try the whole thing as an FQN
/// first, then split on the last dot and resolve the type part through
/// the chain, then look for the member among the type's hard-link
/// children.
///
/// Returns `None` if either step fails. Caller-provided `name` is
/// expected to be a dotted identifier (`A.b.c`) — the function does not
/// itself parse expressions.
pub fn resolve_compound_name<P: AsJava>(
    name: &str,
    imports: &[Import],
    current_package: &str,
    java: &JavaRegistries,
    jvm: &JvmRegistries,
    graph: &Graph<P>,
) -> Option<NodeId> {
    // Try the whole thing as an FQN first.
    if let Some(id) = lookup_fqn(java, jvm, name) {
        return Some(id);
    }

    // Split on the last dot: `Type.method` → ("Type", "method").
    if let Some(dot) = name.rfind('.') {
        let type_part = &name[..dot];
        let member_part = &name[dot + 1..];

        if let Some(type_id) =
            resolve_name(type_part, imports, current_package, java, jvm, graph)
        {
            // Walk the type's hard-link children, filtering for a Java
            // payload whose simple name matches (mirrors the
            // prototype's `lookup_children + filter`).
            let node = graph.get(type_id)?;
            for &child_id in &node.children {
                if let Some(child) = graph.get(child_id)
                    && let Some(child_name) = payload_simple_name(&child.payload)
                    && child_name == member_part
                {
                    return Some(child_id);
                }
            }
        }
    }

    // Fall back to the simple-name chain in case the caller passed an
    // unqualified compound that happened not to split helpfully.
    resolve_name(name, imports, current_package, java, jvm, graph)
}

/// Resolve a fully-qualified name. Prefers the Java-side node when both
/// the language-specific and JVM-projection providers exist (the
/// prototype's `lookup_by_fqn` returned the single declaration node;
/// the Java-side node is the source-level analogue).
pub fn lookup_fqn(java: &JavaRegistries, jvm: &JvmRegistries, fqn: &str) -> Option<NodeId> {
    let java_key = JavaSymbolKey::new(Fqn::new(fqn));
    if let Some(&id) = java.symbols.providers(&java_key).first() {
        return Some(id);
    }

    let type_key = JvmTypeKey::new(Fqn::new(fqn));
    if let Some(&id) = jvm.types.providers(&type_key).first() {
        return Some(id);
    }

    let pkg_key = PackageKey::new(Fqn::new(fqn));
    if let Some(&id) = jvm.packages.providers(&pkg_key).first() {
        return Some(id);
    }

    None
}

/// Walk the graph looking for exactly one Java payload whose simple
/// name matches. Prototype semantics: returns `Some(id)` iff a single
/// match exists. Multiple matches are ambiguous and yield `None`.
///
/// O(n) over the entire graph — only invoked as the last fallback in
/// [`resolve_name`].
pub fn resolve_simple_name<P: AsJava>(name: &str, graph: &Graph<P>) -> Option<NodeId> {
    let mut hits = Vec::new();
    for (id, node) in graph.iter() {
        if let Some(payload_name) = payload_simple_name(&node.payload)
            && payload_name == name
        {
            hits.push(id);
        }
    }
    if hits.len() == 1 {
        Some(hits[0])
    } else {
        None
    }
}

/// Borrow the simple name of a Java payload. JVM-projection nodes
/// return `None` because they're hard-linked siblings of their Java
/// counterparts; resolution always lands on the Java side.
fn payload_simple_name<P: AsJava>(payload: &P) -> Option<&str> {
    payload.as_java().and_then(|j| j.header()).map(|h| h.name.as_str())
}
