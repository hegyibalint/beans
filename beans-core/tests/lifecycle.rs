//! Engine-as-system tests, written as specification.
//!
//! The 338 spec tests in `beans-test-java` check **what Java means**.
//! These tests check **whether the engine works** — what happens across
//! file edits, deletes, re-introductions, and cross-file dependencies.
//! Implementation is disposable; tests are the spec.
//!
//! Three tiers, ordered by how much of the architecture each test
//! anchors:
//!
//! * **Tier 1 — baseline.** What any consumer (LSP, CLI, batch tool)
//!   needs to be true. Failures here are bugs.
//! * **Tier 2 — running engine.** What makes the graph a graph, not a
//!   static snapshot: subscribers fire on lifecycle changes, stale
//!   `NodeId`s do not silently resolve to unrelated payloads.
//! * **Tier 3 — lazy recomputation.** ADR-0009/0010's pull-on-demand
//!   contract. All fail today (no pull function); informs the
//!   path-A-vs-path-B decision documented in the audit.

use std::cell::Cell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use beans_core::graph::{Graph, NodeId};
use beans_core::jvm::keys::{JvmMethodKey, JvmTypeKey};
use beans_core::jvm::{Fqn, JvmNodePayload};
use beans_core::languages::java::{integrate, parse_java_to_graph};
use beans_core::payload::NodePayload;
use beans_core::registries::Registries;
use beans_core::TypeRef;

// =========================================================================
// Test environment
// =========================================================================

/// Minimal "workspace" mimicking what `beans-lsp::ServerState` carries:
/// graph + registries + a per-file root-id map. Independent of
/// `beans-lsp` so these tests stay engine-level.
struct Env {
    graph: Graph<NodePayload>,
    registries: Registries,
    file_roots: HashMap<PathBuf, Vec<NodeId>>,
}

impl Env {
    fn new() -> Self {
        Self {
            graph: Graph::new(),
            registries: Registries::new(),
            file_roots: HashMap::new(),
        }
    }

    /// Parse `source` for `path`, destroy the file's previous roots, and
    /// integrate the new payloads. Models the `did_change` lifecycle.
    fn integrate(&mut self, path: &Path, source: &str) {
        if let Some(old_roots) = self.file_roots.remove(path) {
            for r in old_roots {
                self.graph.destroy(r);
            }
        }
        let parsed = parse_java_to_graph(path, source);
        let inserted = integrate(&mut self.graph, &self.registries, parsed);
        let roots: Vec<NodeId> = inserted
            .iter()
            .copied()
            .filter(|&id| self.graph.get(id).and_then(|n| n.parent).is_none())
            .collect();
        self.file_roots.insert(path.to_path_buf(), roots);
    }

    /// Destroy every root for `path`. Models a file-deletion event.
    fn delete(&mut self, path: &Path) {
        if let Some(roots) = self.file_roots.remove(path) {
            for r in roots {
                self.graph.destroy(r);
            }
        }
    }

    fn types_at(&self, fqn: &str) -> Vec<NodeId> {
        self.registries
            .jvm_types
            .providers(&JvmTypeKey::new(Fqn::new(fqn)))
    }

    fn methods_at(&self, owner: &str, name: &str, params: Vec<TypeRef>) -> Vec<NodeId> {
        self.registries
            .jvm_methods
            .providers(&JvmMethodKey::new(Fqn::new(owner), name, params))
    }
}

// =========================================================================
// Tier 1 — Baseline
// =========================================================================

#[test]
fn integrate_registers_each_declaration() {
    let mut env = Env::new();
    env.integrate(
        Path::new("Service.java"),
        "package com.example; public class Service { public void process() {} }",
    );

    assert_eq!(
        env.types_at("com.example.Service").len(),
        1,
        "type provider registered"
    );
    assert_eq!(
        env.methods_at("com.example.Service", "process", vec![]).len(),
        1,
        "method provider registered"
    );
}

#[test]
fn re_integrate_replaces_old_registrations() {
    let mut env = Env::new();
    let path = Path::new("Service.java");
    env.integrate(
        path,
        "package com.example; public class Service { public void process() {} }",
    );
    assert_eq!(
        env.methods_at("com.example.Service", "process", vec![]).len(),
        1
    );

    env.integrate(
        path,
        "package com.example; public class Service { public void processItem() {} }",
    );

    assert!(
        env.methods_at("com.example.Service", "process", vec![])
            .is_empty(),
        "old method `process` should be gone after reintegration"
    );
    assert_eq!(
        env.methods_at("com.example.Service", "processItem", vec![])
            .len(),
        1,
        "new method `processItem` registered"
    );
}

#[test]
fn delete_clears_all_registrations_for_file() {
    let mut env = Env::new();
    let path = Path::new("Service.java");
    env.integrate(
        path,
        "package com.example; public class Service { public void process() {} }",
    );
    assert!(!env.types_at("com.example.Service").is_empty());

    env.delete(path);

    assert!(
        env.types_at("com.example.Service").is_empty(),
        "type provider cleared on delete"
    );
    assert!(
        env.methods_at("com.example.Service", "process", vec![])
            .is_empty(),
        "method provider cleared on delete"
    );
}

#[test]
fn delete_then_reintroduce_restores_registrations() {
    let mut env = Env::new();
    let path = Path::new("Service.java");
    let source = "package com.example; public class Service { public void process() {} }";
    env.integrate(path, source);
    env.delete(path);
    env.integrate(path, source);

    assert_eq!(env.types_at("com.example.Service").len(), 1);
    assert_eq!(
        env.methods_at("com.example.Service", "process", vec![]).len(),
        1
    );
}

#[test]
fn two_files_can_register_same_type_fqn() {
    // Per ADR-0013 the registry stores all providers; precedence is a
    // resolution-layer concern.
    let mut env = Env::new();
    env.integrate(
        Path::new("a/Foo.java"),
        "package com.example; public class Foo { public void aMethod() {} }",
    );
    env.integrate(
        Path::new("b/Foo.java"),
        "package com.example; public class Foo { public void bMethod() {} }",
    );

    assert_eq!(env.types_at("com.example.Foo").len(), 2);
    assert_eq!(env.methods_at("com.example.Foo", "aMethod", vec![]).len(), 1);
    assert_eq!(env.methods_at("com.example.Foo", "bMethod", vec![]).len(), 1);
}

#[test]
fn deleting_one_provider_leaves_the_other() {
    let mut env = Env::new();
    let a = Path::new("a/Foo.java");
    let b = Path::new("b/Foo.java");
    env.integrate(a, "package com.example; public class Foo {}");
    env.integrate(b, "package com.example; public class Foo {}");
    assert_eq!(env.types_at("com.example.Foo").len(), 2);

    env.delete(a);

    assert_eq!(
        env.types_at("com.example.Foo").len(),
        1,
        "exactly one provider remains after one is deleted"
    );
}

#[test]
fn editing_method_signature_reregisters_under_new_key() {
    // process() and process(String) share an FQN but differ in JvmMethodKey
    // because params differ. Rebuilding the file must move the registration.
    let mut env = Env::new();
    let path = Path::new("Service.java");
    env.integrate(
        path,
        "package com.example; public class Service { public void process() {} }",
    );
    let no_args = vec![];
    let one_arg = vec![TypeRef::Simple {
        name: "String".to_string(),
    }];

    assert_eq!(
        env.methods_at("com.example.Service", "process", no_args.clone())
            .len(),
        1
    );
    assert!(env
        .methods_at("com.example.Service", "process", one_arg.clone())
        .is_empty());

    env.integrate(
        path,
        "package com.example; public class Service { public void process(String s) {} }",
    );

    assert!(
        env.methods_at("com.example.Service", "process", no_args)
            .is_empty(),
        "old no-arg method gone"
    );
    assert_eq!(
        env.methods_at("com.example.Service", "process", one_arg).len(),
        1,
        "new one-arg method registered"
    );
}

// =========================================================================
// Tier 2 — Running engine
//
// These tests pin the contract that makes the graph a *graph* rather than
// a static snapshot: registry mutations propagate to subscribers, dynamic
// links observe changes. Most fail today. Each failure is a precise gap.
// =========================================================================

#[test]
fn registering_a_provider_fires_existing_subscribers() {
    // A consumer subscribed to "com.example.Service" gets told when a
    // file gets indexed providing it. Today: register() does not fire
    // notify(); subscribers stay silent. This test asserts the contract
    // ADR-0014's RAII handles document but the registry doesn't keep.
    let mut env = Env::new();
    let key = JvmTypeKey::new(Fqn::new("com.example.Service"));
    let counter = Rc::new(Cell::new(0u32));
    let cb_counter = counter.clone();
    let _sub = env.registries.jvm_types.subscribe(
        key,
        Rc::new(move || cb_counter.set(cb_counter.get() + 1)),
    );

    env.integrate(
        Path::new("Service.java"),
        "package com.example; public class Service {}",
    );

    assert!(
        counter.get() > 0,
        "subscriber should fire when a provider for its key registers; got {}",
        counter.get()
    );
}

#[test]
fn dropping_a_provider_fires_existing_subscribers() {
    // Mirror: dependent on com.example.Service should be told when the
    // file declaring it is deleted. Today: silent.
    let mut env = Env::new();
    env.integrate(
        Path::new("Service.java"),
        "package com.example; public class Service {}",
    );

    let key = JvmTypeKey::new(Fqn::new("com.example.Service"));
    let counter = Rc::new(Cell::new(0u32));
    let cb_counter = counter.clone();
    let _sub = env.registries.jvm_types.subscribe(
        key,
        Rc::new(move || cb_counter.set(cb_counter.get() + 1)),
    );

    env.delete(Path::new("Service.java"));

    assert!(
        counter.get() > 0,
        "subscriber should fire when its provider drops; got {}",
        counter.get()
    );
}

#[test]
fn stale_node_id_does_not_resolve_to_a_different_payload() {
    // ABA hazard: if Service.java is destroyed and re-integrated, the
    // freed slot may be reused. A consumer holding the *old* NodeId
    // could resolve to a *different* declaration that happened to
    // recycle the slot. The engine must either preserve identity across
    // reintegration or prevent stale ids from resolving to unrelated
    // payloads.
    //
    // Today: NodeId is a raw arena slot index reused via free-list.
    // This test pins the desired property: a stale id is either gone or
    // points at the same logical declaration it did before.
    let mut env = Env::new();
    let path = Path::new("Service.java");
    env.integrate(
        path,
        "package com.example; public class Service { public void process() {} }",
    );
    let stale_id = env
        .methods_at("com.example.Service", "process", vec![])
        .first()
        .copied()
        .expect("process registered");

    // Edit the file: process() becomes processItem(). The old slot is freed.
    env.integrate(
        path,
        "package com.example; public class Service { public void processItem() {} }",
    );

    if let Some(node) = env.graph.get(stale_id) {
        match &node.payload {
            NodePayload::Jvm(JvmNodePayload::Method(m)) => {
                assert_eq!(
                    m.header.name, "process",
                    "stale id resolved to a method named {}, not the original `process`",
                    m.header.name
                );
            }
            other => panic!(
                "stale id resolved to a non-method payload: {:?}",
                std::mem::discriminant(other)
            ),
        }
    }
    // If env.graph.get(stale_id) returns None, that's the safer outcome.
}

// =========================================================================
// Tier 3 — Lazy recomputation
//
// ADR-0009/0010 promise: stale → pull → recompute. None of this exists
// today. Tests document what the runtime owes the architecture; bodies
// are sketches because the API isn't there to call.
// =========================================================================

#[test]
#[ignore = "no Graph::pull function exists; ADR-0009/0010 not implemented"]
fn pull_on_stale_node_recomputes_value() {
    // Spec: a node marked stale, when pulled, transitions Stale →
    // Computing → Fresh(g) and returns the recomputed value. Today: no
    // pull function.
    //
    // Sketch:
    //   let id = env.insert_some_computed_node();
    //   env.graph.mark_stale(id);
    //   let val = env.graph.pull(id);  // <-- doesn't exist
    //   assert_eq!(env.graph.get(id).unwrap().state, CacheState::Fresh(_));
    //   assert!(val.is_some());
}

#[test]
#[ignore = "lazy recomputation not implemented; cannot measure unobserved-stale cost"]
fn unobserved_stale_node_does_not_recompute() {
    // Spec (ADR-0010): a stale node nobody pulls stays stale forever.
    // The runtime tracks recompute count; the test asserts it stays at
    // zero for stale nodes that no consumer queries.
}

#[test]
#[ignore = "generation-based freshness comparison not implemented"]
fn fresh_node_with_stale_dependency_recomputes_on_pull() {
    // Spec: on pull, walk dependencies; if any dep was bumped since this
    // node's `Fresh(g)` stamp, recompute even if our state says Fresh.
    // Today the Generation field on `Fresh` is never compared.
}

#[test]
#[ignore = "Computing-state cycle detection not exercised by any caller"]
fn pull_on_self_referential_node_does_not_loop() {
    // Spec: if the pull walk encounters a node already in `Computing`
    // state, it returns a cycle marker rather than recursing. Today
    // `Computing` is a CacheState variant nothing reads.
}
