//! Typed registry keys for the JVM projection.
//!
//! Per ADR-0012 each registry has its own typed key — there is no shared
//! `RegistryKey` enum. Five JVM-shaped keys cover the bytecode-level
//! lookups every JVM language eventually needs: types (classes,
//! interfaces, enums, records, annotations), fields, methods,
//! constructors, and packages.
//!
//! The keys are deliberately small and `Hash`/`Eq` so they live cheaply
//! inside dynamic-link query lists (per ADR-0008's "millions of links"
//! note). Method and constructor keys carry an erased parameter list as
//! [`Vec<TypeRef>`]; per JLS §4.6 method overload resolution at the JVM
//! level uses erased descriptors, so the keys compare on erased types.
//! It is the producer's responsibility to pre-erase — the registry layer
//! is dumb (ADR-0013).

use crate::jvm::fqn::Fqn;
use crate::jvm::type_ref::TypeRef;

/// Key identifying a JVM type (class, interface, enum, record, annotation
/// type) by its fully-qualified name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JvmTypeKey {
    pub fqn: Fqn,
}

impl JvmTypeKey {
    pub fn new(fqn: impl Into<Fqn>) -> Self {
        Self { fqn: fqn.into() }
    }
}

/// Key identifying a JVM field by its declaring type and simple name.
///
/// Two fields cannot share a `(owner, name)` pair on the same class
/// (JLS §8.3), so this is sufficient identity at the bytecode level.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JvmFieldKey {
    pub owner: Fqn,
    pub name: String,
}

impl JvmFieldKey {
    pub fn new(owner: impl Into<Fqn>, name: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            name: name.into(),
        }
    }
}

/// Key identifying a JVM method by its declaring type, simple name, and
/// erased parameter list.
///
/// Per JLS §8.4.2 two methods with the same `(name, erased-params)` on
/// the same class are duplicates; this key is therefore unique up to
/// overload.
///
/// **Producer obligations.** The `param_types` `Vec<TypeRef>` must be
/// constructed with both:
///
/// 1. **Erasure applied** — pre-erase via
///    [`TypeRef::erasure`](crate::jvm::TypeRef::erasure) (JLS §4.6) so
///    `List<String>` and `List<Integer>` collapse to the same key.
/// 2. **Fully-qualified `Simple` names** — `TypeRef::Simple { name:
///    "java.lang.String" }`, never `Simple { name: "String" }`. Two
///    producers that disagree on `String` vs `java.lang.String` will
///    register identical-looking methods under different keys, and
///    cross-file resolution will silently miss.
///
/// Per ADR-0013 the registry layer is dumb and does not normalise either
/// of these — both are the producer's responsibility. The JVM-projection
/// emit path in `languages/java/parser.rs` is the canonical example.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JvmMethodKey {
    pub owner: Fqn,
    pub name: String,
    pub param_types: Vec<TypeRef>,
}

impl JvmMethodKey {
    pub fn new(
        owner: impl Into<Fqn>,
        name: impl Into<String>,
        param_types: Vec<TypeRef>,
    ) -> Self {
        Self {
            owner: owner.into(),
            name: name.into(),
            param_types,
        }
    }
}

/// Key identifying a JVM constructor by its declaring type and erased
/// parameter list.
///
/// Constructors are not modelled as named methods because the JVM
/// represents them differently (`<init>` byte-name, no return type
/// dispatch); keeping the key separate from [`JvmMethodKey`] means
/// constructor lookups don't have to thread a sentinel name through.
///
/// The same producer obligations as [`JvmMethodKey`] apply to
/// `param_types`: erased and fully-qualified.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JvmConstructorKey {
    pub owner: Fqn,
    pub param_types: Vec<TypeRef>,
}

impl JvmConstructorKey {
    pub fn new(owner: impl Into<Fqn>, param_types: Vec<TypeRef>) -> Self {
        Self {
            owner: owner.into(),
            param_types,
        }
    }
}

/// Key identifying a package by its dotted name (JLS §7.4).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageKey {
    pub package: Fqn,
}

impl PackageKey {
    pub fn new(package: impl Into<Fqn>) -> Self {
        Self {
            package: package.into(),
        }
    }
}
