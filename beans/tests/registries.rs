//! End-to-end exercises for the JVM and Java registries.
//!
//! Build node payloads by hand (no parser), register them with the
//! typed-key registries (ADR-0012 / ADR-0013), and prove the cross-
//! language Java → JVM fallback shape ADR-0008 describes works through
//! [`FallbackSubscription`] — the concrete two-key watch with primary-
//! then-fallback resolve semantics.
//!
//! Multi-registry merge-all (the completion-style "show every match
//! across N registries") is not exercised here; that pattern materializes
//! as a separate concrete composition (e.g., a future `CompletionLookup`)
//! when a real consumer needs it.

use beans::graph::Graph;
use beans::jvm::{
    Fqn, JvmDeclHeader, JvmEnrichments, JvmMethodKey, JvmMethodNode, JvmNodePayload, JvmParameter,
    JvmTypeKey, JvmTypeKind, JvmTypeNode, NullabilityInfo, PackageKey, TypeRef,
};
use beans::languages::java::{
    JavaDeclHeader, JavaNodePayload, JavaSymbolKey, JavaTypeKind, JavaTypeNode,
};
use beans::{FallbackSubscription, NodePayload, QueryResult, Registries};

// --- Payload helpers ---

fn java_class(name: &str, fqn: &str) -> NodePayload {
    NodePayload::Java(JavaNodePayload::from(JavaTypeNode {
        header: JavaDeclHeader::new(name, fqn),
        kind: JavaTypeKind::Class,
        type_parameters: vec![],
        record_components: vec![],
    }))
}

fn jvm_type(name: &str, fqn: &str, kind: JvmTypeKind) -> NodePayload {
    NodePayload::Jvm(JvmNodePayload::from(JvmTypeNode {
        header: JvmDeclHeader::new(name, fqn),
        kind,
        type_parameters: vec![],
        record_components: vec![],
        enrichments: JvmEnrichments::default(),
    }))
}

fn jvm_method(name: &str, fqn: &str, owner: &str, return_type: TypeRef) -> NodePayload {
    NodePayload::Jvm(JvmNodePayload::from(JvmMethodNode {
        header: JvmDeclHeader::new(name, fqn),
        owner: owner.into(),
        return_type,
        parameters: vec![],
        type_parameters: vec![],
        throws: vec![],
        enrichments: JvmEnrichments::default(),
    }))
}

// --- Tests ---

#[test]
fn java_type_resolves_through_java_registry_first() {
    // Java side has the type; the JVM projection has it too. The
    // FallbackSubscription's primary (Java) wins.
    let mut graph: Graph<NodePayload> = Graph::new();
    let registries = Registries::new();

    let java_id = graph.insert(java_class("Service", "com.example.Service"), None);
    let _java_h = registries
        .java.symbols
        .register(JavaSymbolKey::new("com.example.Service"), java_id);

    let jvm_id = graph.insert(
        jvm_type("Service", "com.example.Service", JvmTypeKind::Class),
        Some(java_id), // hard-linked projection child of Java node.
    );
    let _jvm_h = registries
        .jvm.types
        .register(JvmTypeKey::new("com.example.Service"), jvm_id);

    let fb: FallbackSubscription<JavaSymbolKey, JvmTypeKey> = FallbackSubscription::new(
        &registries.java.symbols,
        JavaSymbolKey::new("com.example.Service"),
        &registries.jvm.types,
        JvmTypeKey::new("com.example.Service"),
    );

    assert_eq!(fb.resolve().first(), Some(java_id));
}

#[test]
fn falls_through_to_jvm_when_no_java_provider_exists() {
    // Cross-language case: the type is defined in Kotlin (modelled here
    // as "no Java provider, only a JVM projection"). The Java-side
    // primary misses; the FallbackSubscription falls through to JVM.
    let mut graph: Graph<NodePayload> = Graph::new();
    let registries = Registries::new();

    let jvm_id = graph.insert(
        jvm_type("Service", "com.example.Service", JvmTypeKind::Class),
        None,
    );
    let _jvm_h = registries
        .jvm.types
        .register(JvmTypeKey::new("com.example.Service"), jvm_id);

    let fb: FallbackSubscription<JavaSymbolKey, JvmTypeKey> = FallbackSubscription::new(
        &registries.java.symbols,
        JavaSymbolKey::new("com.example.Service"),
        &registries.jvm.types,
        JvmTypeKey::new("com.example.Service"),
    );

    assert_eq!(fb.resolve().first(), Some(jvm_id));
}

#[test]
fn unresolved_when_neither_registry_has_provider() {
    let registries = Registries::new();

    let fb: FallbackSubscription<JavaSymbolKey, JvmTypeKey> = FallbackSubscription::new(
        &registries.java.symbols,
        JavaSymbolKey::new("missing.Type"),
        &registries.jvm.types,
        JvmTypeKey::new("missing.Type"),
    );

    assert!(matches!(fb.resolve(), QueryResult::None));
}

#[test]
fn fallback_observes_jvm_projection_arriving_after_construction() {
    // Tier-2 contract via FallbackSubscription: the cache invalidates
    // when *either* underlying registry's provider set changes, and the
    // fallback path picks up the new state on next resolve. No manual
    // invalidate.
    let registries = Registries::new();

    let fb: FallbackSubscription<JavaSymbolKey, JvmTypeKey> = FallbackSubscription::new(
        &registries.java.symbols,
        JavaSymbolKey::new("com.example.Late"),
        &registries.jvm.types,
        JvmTypeKey::new("com.example.Late"),
    );
    assert!(fb.resolve().is_empty());

    let mut graph: Graph<NodePayload> = Graph::new();
    let late_id = graph.insert(
        jvm_type("Late", "com.example.Late", JvmTypeKind::Class),
        None,
    );
    let _h = registries
        .jvm.types
        .register(JvmTypeKey::new("com.example.Late"), late_id);

    assert_eq!(fb.resolve().first(), Some(late_id));
}

#[test]
fn method_overload_keys_distinguish_by_param_types() {
    // Two overloads of `process` on the same owner; the JVM registry
    // distinguishes them because `JvmMethodKey` includes the erased
    // parameter list (per JLS §8.4.2 / ADR-0012).
    let mut graph: Graph<NodePayload> = Graph::new();
    let registries = Registries::new();

    let owner = "com.example.Service";
    let int_id = graph.insert(
        jvm_method("process", "com.example.Service.process", owner, TypeRef::Void),
        None,
    );
    let str_id = graph.insert(
        jvm_method("process", "com.example.Service.process", owner, TypeRef::Void),
        None,
    );

    let int_key = JvmMethodKey::new(
        owner,
        "process",
        vec![TypeRef::Primitive(beans::PrimitiveKind::Int)],
    );
    let str_key = JvmMethodKey::new(
        owner,
        "process",
        vec![TypeRef::simple("java.lang.String")],
    );

    let _hi = registries.jvm.methods.register(int_key.clone(), int_id);
    let _hs = registries.jvm.methods.register(str_key.clone(), str_id);

    assert_eq!(registries.jvm.methods.providers(&int_key), vec![int_id]);
    assert_eq!(registries.jvm.methods.providers(&str_key), vec![str_id]);
}

#[test]
fn package_registry_isolated_from_type_registry() {
    // Per ADR-0012 packages are their own registry, not a tagged type.
    // A `PackageKey("com.example")` and `JvmTypeKey("com.example")`
    // never collide, even with the same dotted string.
    let mut graph: Graph<NodePayload> = Graph::new();
    let registries = Registries::new();

    let pkg_payload =
        NodePayload::Jvm(JvmNodePayload::from(beans::jvm::JvmPackageNode {
            header: JvmDeclHeader::new("com.example", "com.example"),
        }));
    let pkg_id = graph.insert(pkg_payload, None);
    let _hp = registries
        .jvm.packages
        .register(PackageKey::new("com.example"), pkg_id);

    assert_eq!(
        registries
            .jvm.packages
            .providers(&PackageKey::new("com.example")),
        vec![pkg_id]
    );
    assert!(
        registries
            .jvm.types
            .providers(&JvmTypeKey::new("com.example"))
            .is_empty()
    );
}

#[test]
fn enrichments_default_to_none_for_java_sources() {
    let payload = jvm_method(
        "process",
        "com.example.Service.process",
        "com.example.Service",
        TypeRef::Void,
    );
    if let NodePayload::Jvm(JvmNodePayload::Method(node)) = payload {
        assert!(node.enrichments.nullability.is_none());
    } else {
        panic!("expected JvmNodePayload::Method");
    }

    let param = JvmParameter {
        name: "input".to_string(),
        param_type: TypeRef::simple("java.lang.String"),
        is_varargs: false,
        enrichments: JvmEnrichments {
            nullability: Some(NullabilityInfo::NonNull),
        },
    };
    assert_eq!(param.enrichments.nullability, Some(NullabilityInfo::NonNull));
}

#[test]
fn fqn_round_trips_through_keys() {
    let key1 = JvmTypeKey::new("com.example.Service");
    let key2 = JvmTypeKey::new(Fqn::new("com.example.Service"));
    let key3 = JvmTypeKey::new("com.example.Service".to_string());
    assert_eq!(key1, key2);
    assert_eq!(key2, key3);
}

#[test]
fn registry_returns_all_providers_for_a_key() {
    // ADR-0013: a registry stores *all* providers; a key with two
    // registrations returns both via `providers`. (Registry::providers
    // is the raw form; QueryResult::Many would model the same fact
    // via Registry::query.)
    let mut graph: Graph<NodePayload> = Graph::new();
    let registries = Registries::new();

    let id1 = graph.insert(
        java_class("Shared", "com.example.Shared"),
        None,
    );
    let id2 = graph.insert(
        java_class("Shared", "com.example.Shared"),
        None,
    );
    let _h1 = registries
        .java.symbols
        .register(JavaSymbolKey::new("com.example.Shared"), id1);
    let _h2 = registries
        .java.symbols
        .register(JavaSymbolKey::new("com.example.Shared"), id2);

    let providers = registries
        .java.symbols
        .providers(&JavaSymbolKey::new("com.example.Shared"));
    assert_eq!(providers, vec![id1, id2]);
}
