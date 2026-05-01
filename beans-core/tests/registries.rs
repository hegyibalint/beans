//! End-to-end exercises for the JVM and Java registries.
//!
//! Build node payloads by hand (no parser), register them with the
//! typed-key registries (ADR-0012 / ADR-0013), and prove the cross-
//! language Java → JVM fallback shape ADR-0008 describes works as a
//! priority-ordered iterator chain over [`RegistryQuery`] impls. The
//! eventual `MultiQuery<Q>` abstraction (when use-site nodes need
//! caching/subscriptions) will wrap this same iter shape.

use beans_core::graph::{Graph, NodeId, RegistryQuery};
use beans_core::jvm::{
    Fqn, JvmDeclHeader, JvmEnrichments, JvmFieldKey, JvmFieldNode, JvmMethodKey, JvmMethodNode,
    JvmNodePayload, JvmParameter, JvmTypeKey, JvmTypeKind, JvmTypeNode, NullabilityInfo,
    PackageKey, TypeRef,
};
use beans_core::languages::java::{
    JavaDeclHeader, JavaMethodNode, JavaNodePayload, JavaSymbolKey, JavaTypeKind, JavaTypeNode,
};
use beans_core::{NodePayload, Registries};

// --- Query enums ---
//
// These are what a real language module would define for use-site links.
// Keeping them in the test file rather than in `beans-core` itself
// matches the contract of step 3: the engine ships keys + registries +
// the `RegistryQuery` trait; the language modules build their own query
// enums on top.

#[derive(Debug, Clone)]
enum TypeQuery {
    Java(JavaSymbolKey),
    Jvm(JvmTypeKey),
}

impl RegistryQuery for TypeQuery {
    type Ctx = Registries;
    fn resolve(&self, ctx: &Self::Ctx) -> Vec<NodeId> {
        match self {
            TypeQuery::Java(k) => ctx.java_symbols.providers(k),
            TypeQuery::Jvm(k) => ctx.jvm_types.providers(k),
        }
    }
}

#[derive(Debug, Clone)]
enum MemberQuery {
    Java(JavaSymbolKey),
    JvmMethod(JvmMethodKey),
    JvmField(JvmFieldKey),
}

impl RegistryQuery for MemberQuery {
    type Ctx = Registries;
    fn resolve(&self, ctx: &Self::Ctx) -> Vec<NodeId> {
        match self {
            MemberQuery::Java(k) => ctx.java_symbols.providers(k),
            MemberQuery::JvmMethod(k) => ctx.jvm_methods.providers(k),
            MemberQuery::JvmField(k) => ctx.jvm_fields.providers(k),
        }
    }
}

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
    // priority-ordered iter chain `[Java, Jvm].flat_map(resolve).next()`
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

    let queries = [
        TypeQuery::Java(JavaSymbolKey::new("com.example.Service")),
        TypeQuery::Jvm(JvmTypeKey::new("com.example.Service")),
    ];

    let resolved = queries.iter().flat_map(|q| q.resolve(&registries)).next();
    assert_eq!(resolved, Some(java_id));
}

#[test]
fn falls_through_to_jvm_when_no_java_provider_exists() {
    // Cross-language case: the type is defined in Kotlin (modelled here
    // as "no Java provider, only a JVM projection"). The Java-side
    // query misses; the iter chain falls through to the JVM query.
    let mut graph: Graph<NodePayload> = Graph::new();
    let registries = Registries::new();

    // Only the JVM projection is registered.
    let jvm_id = graph.insert(
        jvm_type("Service", "com.example.Service", JvmTypeKind::Class),
        None,
    );
    let _jvm_h = registries
        .jvm_types
        .register(JvmTypeKey::new("com.example.Service"), jvm_id);

    let queries = [
        TypeQuery::Java(JavaSymbolKey::new("com.example.Service")),
        TypeQuery::Jvm(JvmTypeKey::new("com.example.Service")),
    ];

    let resolved = queries.iter().flat_map(|q| q.resolve(&registries)).next();
    assert_eq!(resolved, Some(jvm_id));
}

#[test]
fn unresolved_when_neither_registry_has_provider() {
    let registries = Registries::new();

    let queries = [
        TypeQuery::Java(JavaSymbolKey::new("missing.Type")),
        TypeQuery::Jvm(JvmTypeKey::new("missing.Type")),
    ];

    let resolved = queries.iter().flat_map(|q| q.resolve(&registries)).next();
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
    // Java methods, JVM-projected methods, JVM-projected fields. The
    // MergeAll mode runs every query and concatenates.
    let mut graph: Graph<NodePayload> = Graph::new();
    let registries = Registries::new();

    let owner = "com.example.Service";

    let java_id = graph.insert(
        java_method("process", "com.example.Service.process"),
        None,
    );
    let _java_h = registries
        .java_symbols
        .register(JavaSymbolKey::new("com.example.Service.process"), java_id);

    let jvm_method_id = graph.insert(
        jvm_method("process", "com.example.Service.process", owner, TypeRef::Void),
        Some(java_id),
    );
    let _jvm_h = registries.jvm_methods.register(
        JvmMethodKey::new(owner, "process", vec![]),
        jvm_method_id,
    );

    let field_id = graph.insert(
        jvm_field(
            "name",
            "com.example.Service.name",
            owner,
            TypeRef::simple("java.lang.String"),
        ),
        None,
    );
    let _field_h = registries
        .jvm_fields
        .register(JvmFieldKey::new(owner, "name"), field_id);

    let queries = [
        MemberQuery::Java(JavaSymbolKey::new("com.example.Service.process")),
        MemberQuery::JvmMethod(JvmMethodKey::new(owner, "process", vec![])),
        MemberQuery::JvmField(JvmFieldKey::new(owner, "name")),
    ];

    let results: Vec<NodeId> = queries
        .iter()
        .flat_map(|q| q.resolve(&registries))
        .collect();
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

    // No type with the dotted-FQN "com.example" — there shouldn't be one
    // (it's a package, not a type), and the type registry must reflect
    // that.
    assert_eq!(registries.jvm_packages.providers(&PackageKey::new("com.example")), vec![pkg_id]);
    assert!(
        registries
            .jvm_types
            .providers(&JvmTypeKey::new("com.example"))
            .is_empty()
    );
}

#[test]
fn enrichments_default_to_none_for_java_sources() {
    // ADR-0004 promotes nullability onto the JVM projection, but a
    // Java-sourced JVM node has no source-language opinion, so its
    // enrichments must default to None.
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

    // The shape exists on parameters too. Round-trip a parameter with
    // an explicit nullability fact to prove the field is wired through.
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
    // Sanity: keys store Fqn, but accept anything that converts to it
    // (including &str). This keeps producer code readable without
    // forcing every call site to spell out `Fqn::new(...)`.
    let key1 = JvmTypeKey::new("com.example.Service");
    let key2 = JvmTypeKey::new(Fqn::new("com.example.Service"));
    let key3 = JvmTypeKey::new("com.example.Service".to_string());
    assert_eq!(key1, key2);
    assert_eq!(key2, key3);
}

#[test]
fn merge_all_returns_duplicates_when_same_node_hits_multiple_queries() {
    // Per ADR-0008 the MergeAll combine mode does NOT dedup. If the same
    // NodeId is registered under two queries on the link's list — for
    // example a node that provides both a Java-side key and its JVM
    // projection key — `resolve_all` returns it once per hit, in
    // priority order. The consumer is responsible for collapsing
    // duplicates with knowledge of which language wins (ADR-0013: the
    // registry layer is dumb).
    let mut graph: Graph<NodePayload> = Graph::new();
    let registries = Registries::new();

    // Register the same NodeId in both registries. This models a node
    // that "is its own JVM projection" — useful as a stress test for the
    // dedup contract even if the parser typically uses two distinct
    // NodeIds (one Java, one JVM-projection child).
    let id = graph.insert(
        java_class("Shared", "com.example.Shared"),
        None,
    );
    let _hj = registries
        .java_symbols
        .register(JavaSymbolKey::new("com.example.Shared"), id);
    let _ht = registries
        .jvm_types
        .register(JvmTypeKey::new("com.example.Shared"), id);

    let queries = [
        TypeQuery::Java(JavaSymbolKey::new("com.example.Shared")),
        TypeQuery::Jvm(JvmTypeKey::new("com.example.Shared")),
    ];

    // Two hits, same NodeId — the iter chain returns both, no dedup.
    // Consumers that want one entry per logical symbol (typical for
    // completion) collapse downstream. ADR-0008 commits to dedup as a
    // future MultiQuery<Q> responsibility, not the registry layer's
    // (ADR-0013: registry is dumb).
    let merged: Vec<NodeId> = queries
        .iter()
        .flat_map(|q| q.resolve(&registries))
        .collect();
    assert_eq!(merged, vec![id, id]);
}
