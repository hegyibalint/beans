//! End-to-end exercises for the JVM and Java registries.
//!
//! Build node payloads by hand (no parser), register them with the
//! typed-key registries (ADR-0012 / ADR-0013), and prove the cross-
//! language Java → JVM fallback shape ADR-0008 describes. Two flavours
//! of cross-registry consumption are exercised:
//!
//! - [`first_match`] / [`all_matches`] over `&[&dyn Queryable<M>]` for
//!   stateless one-shot queries (this file's `TypeQuery`-shaped tests).
//! - [`MultiQuery`] for stored, subscription-backed queries (the
//!   merge-all completion case here is a one-shot use of the
//!   underlying [`RegistryQuery`] enum; MultiQuery's own tests live in
//!   `beans-core/src/multi_query.rs`).

use beans_core::graph::{Graph, NodeId};
use beans_core::jvm::{
    Fqn, JvmDeclHeader, JvmEnrichments, JvmFieldKey, JvmFieldNode, JvmMethodKey, JvmMethodNode,
    JvmNodePayload, JvmParameter, JvmTypeKey, JvmTypeKind, JvmTypeNode, NullabilityInfo,
    PackageKey, TypeRef,
};
use beans_core::languages::java::{
    JavaDeclHeader, JavaMethodNode, JavaNodePayload, JavaSymbolKey, JavaTypeKind, JavaTypeNode,
};
use beans_core::{
    all_matches, first_match, ByFqn, NodePayload, Registries, RegistryQuery,
};

// --- Payload helpers ---

fn java_class(name: &str, fqn: &str) -> NodePayload {
    NodePayload::Java(JavaNodePayload::Type(JavaTypeNode {
        header: JavaDeclHeader::new(name, fqn),
        kind: JavaTypeKind::Class,
        type_parameters: vec![],
        record_components: vec![],
    }))
}

fn java_method(name: &str, fqn: &str) -> NodePayload {
    NodePayload::Java(JavaNodePayload::Method(JavaMethodNode {
        header: JavaDeclHeader::new(name, fqn),
        return_type: TypeRef::Void,
        parameters: vec![],
        type_parameters: vec![],
        throws: vec![],
    }))
}

fn jvm_type(name: &str, fqn: &str, kind: JvmTypeKind) -> NodePayload {
    NodePayload::Jvm(JvmNodePayload::Type(JvmTypeNode {
        header: JvmDeclHeader::new(name, fqn),
        kind,
        type_parameters: vec![],
        record_components: vec![],
        enrichments: JvmEnrichments::default(),
    }))
}

fn jvm_method(name: &str, fqn: &str, owner: &str, return_type: TypeRef) -> NodePayload {
    NodePayload::Jvm(JvmNodePayload::Method(JvmMethodNode {
        header: JvmDeclHeader::new(name, fqn),
        owner: owner.into(),
        return_type,
        parameters: vec![],
        type_parameters: vec![],
        throws: vec![],
        enrichments: JvmEnrichments::default(),
    }))
}

fn jvm_field(name: &str, fqn: &str, owner: &str, field_type: TypeRef) -> NodePayload {
    NodePayload::Jvm(JvmNodePayload::Field(JvmFieldNode {
        header: JvmDeclHeader::new(name, fqn),
        owner: owner.into(),
        field_type,
        constant_value: None,
        initialized: false,
        enrichments: JvmEnrichments::default(),
    }))
}

// --- Tests ---

#[test]
fn java_type_resolves_through_java_registry_first() {
    // Java side has the type; the JVM projection has it too. The
    // priority-ordered first_match across [java_symbols, jvm_types]
    // picks the Java node.
    let mut graph: Graph<NodePayload> = Graph::new();
    let registries = Registries::new();

    let java_id = graph.insert(java_class("Service", "com.example.Service"), None);
    let _java_h = registries
        .java_symbols
        .register(JavaSymbolKey::new("com.example.Service"), java_id);

    let jvm_id = graph.insert(
        jvm_type("Service", "com.example.Service", JvmTypeKind::Class),
        Some(java_id), // hard-linked projection child of Java node.
    );
    let _jvm_h = registries
        .jvm_types
        .register(JvmTypeKey::new("com.example.Service"), jvm_id);

    let resolved = first_match::<ByFqn>(
        &ByFqn(Fqn::new("com.example.Service")),
        &[&registries.java_symbols, &registries.jvm_types],
    );
    assert_eq!(resolved, Some(java_id));
}

#[test]
fn falls_through_to_jvm_when_no_java_provider_exists() {
    // Cross-language case: the type is defined in Kotlin (modelled here
    // as "no Java provider, only a JVM projection"). The Java-side
    // query misses; first_match falls through to the JVM query.
    let mut graph: Graph<NodePayload> = Graph::new();
    let registries = Registries::new();

    let jvm_id = graph.insert(
        jvm_type("Service", "com.example.Service", JvmTypeKind::Class),
        None,
    );
    let _jvm_h = registries
        .jvm_types
        .register(JvmTypeKey::new("com.example.Service"), jvm_id);

    let resolved = first_match::<ByFqn>(
        &ByFqn(Fqn::new("com.example.Service")),
        &[&registries.java_symbols, &registries.jvm_types],
    );
    assert_eq!(resolved, Some(jvm_id));
}

#[test]
fn unresolved_when_neither_registry_has_provider() {
    let registries = Registries::new();

    let resolved = first_match::<ByFqn>(
        &ByFqn(Fqn::new("missing.Type")),
        &[&registries.java_symbols, &registries.jvm_types],
    );
    assert_eq!(resolved, None);
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
        vec![TypeRef::Primitive(beans_core::PrimitiveKind::Int)],
    );
    let str_key = JvmMethodKey::new(
        owner,
        "process",
        vec![TypeRef::simple("java.lang.String")],
    );

    let _hi = registries.jvm_methods.register(int_key.clone(), int_id);
    let _hs = registries.jvm_methods.register(str_key.clone(), str_id);

    assert_eq!(registries.jvm_methods.providers(&int_key), vec![int_id]);
    assert_eq!(registries.jvm_methods.providers(&str_key), vec![str_id]);
}

#[test]
fn merge_all_unions_java_and_jvm_completions_in_priority_order() {
    // Completion at `service.<cur>` wants every plausible candidate:
    // Java methods, JVM-projected methods, JVM-projected fields. These
    // hit different registries with different key types, so the
    // closed-enum `RegistryQuery` is the right shape — each variant
    // carries its typed key.
    use beans_core::Beans;

    let mut beans = Beans::new();
    let owner = "com.example.Service";

    let java_id = beans
        .graph
        .insert(java_method("process", "com.example.Service.process"), None);
    let _java_h = beans
        .registries
        .java_symbols
        .register(JavaSymbolKey::new("com.example.Service.process"), java_id);

    let jvm_method_id = beans.graph.insert(
        jvm_method("process", "com.example.Service.process", owner, TypeRef::Void),
        Some(java_id),
    );
    let _jvm_h = beans.registries.jvm_methods.register(
        JvmMethodKey::new(owner, "process", vec![]),
        jvm_method_id,
    );

    let field_id = beans.graph.insert(
        jvm_field(
            "name",
            "com.example.Service.name",
            owner,
            TypeRef::simple("java.lang.String"),
        ),
        None,
    );
    let _field_h = beans
        .registries
        .jvm_fields
        .register(JvmFieldKey::new(owner, "name"), field_id);

    let queries = [
        RegistryQuery::JavaSymbol(JavaSymbolKey::new("com.example.Service.process")),
        RegistryQuery::JvmMethod(JvmMethodKey::new(owner, "process", vec![])),
        RegistryQuery::JvmField(JvmFieldKey::new(owner, "name")),
    ];
    let results: Vec<NodeId> = queries.iter().flat_map(|q| q.providers(&beans)).collect();
    assert_eq!(results, vec![java_id, jvm_method_id, field_id]);
}

#[test]
fn package_registry_isolated_from_type_registry() {
    // Per ADR-0012 packages are their own registry, not a tagged type.
    // A `PackageKey("com.example")` and `JvmTypeKey("com.example")`
    // never collide, even with the same dotted string.
    let mut graph: Graph<NodePayload> = Graph::new();
    let registries = Registries::new();

    let pkg_payload =
        NodePayload::Jvm(JvmNodePayload::Package(beans_core::jvm::JvmPackageNode {
            header: JvmDeclHeader::new("com.example", "com.example"),
        }));
    let pkg_id = graph.insert(pkg_payload, None);
    let _hp = registries
        .jvm_packages
        .register(PackageKey::new("com.example"), pkg_id);

    assert_eq!(
        registries
            .jvm_packages
            .providers(&PackageKey::new("com.example")),
        vec![pkg_id]
    );
    assert!(
        registries
            .jvm_types
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
fn all_matches_returns_duplicates_when_same_node_hits_multiple_queries() {
    // Per ADR-0008 the MergeAll combine mode does NOT dedup. If the same
    // NodeId is registered under two queries — e.g. a node that provides
    // both a Java-side key and its JVM projection key — `all_matches`
    // returns it once per hit, in priority order. The consumer collapses
    // duplicates with knowledge of which language wins (ADR-0013: the
    // registry layer is dumb).
    let mut graph: Graph<NodePayload> = Graph::new();
    let registries = Registries::new();

    let id = graph.insert(java_class("Shared", "com.example.Shared"), None);
    let _hj = registries
        .java_symbols
        .register(JavaSymbolKey::new("com.example.Shared"), id);
    let _ht = registries
        .jvm_types
        .register(JvmTypeKey::new("com.example.Shared"), id);

    let merged = all_matches::<ByFqn>(
        &ByFqn(Fqn::new("com.example.Shared")),
        &[&registries.java_symbols, &registries.jvm_types],
    );
    assert_eq!(merged, vec![id, id]);
}
