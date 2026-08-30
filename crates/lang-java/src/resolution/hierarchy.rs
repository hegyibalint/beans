//! JLS §8.2 and §9.2: the members a type has that it did not declare.
//!
//! Every other module here answers a question about one `model::File`. This one
//! cannot, and that is the whole point of it. A supertype is usually in another
//! file, and past the edge of the source tree it is not a file at all — `Object`
//! is a class file in a runtime image, and so is everything between a record or
//! an enum and it. A walk that stops where the sources stop stops one hop short
//! of every Java program.
//!
//! So this module speaks `TypeTarget`, which already draws the only distinction
//! that matters: a file we parsed hands back an index into its model, and a
//! class file hands back a binary name and the decoded declaration behind it.
//! The walk does not care which it is holding.

use std::collections::HashSet;

use beans_platform_jvm as jvm;

use crate::{model, query::Query};

use super::{TypeTarget, Wanted, methods::members_of, types::resolve_type_reference};

/// One member of a type, from whichever half of the lake held it.
///
/// The same split `TypeTarget` draws, one level down. A caller that wants a name
/// and a shape reads both arms; one that wants a span to jump to reads only the
/// first, because a class file has no source to point at.
#[derive(Debug, Clone)]
pub(crate) enum Member {
    Parsed {
        source: jvm::model::Source,
        declaration: model::DeclarationId,
    },
    Compiled {
        source: jvm::model::Source,
        /// The type that declares it, kept because §6.6.1 asks which package a
        /// package-private member came from and the member itself cannot say.
        owner: jvm::model::BinaryName,
        declaration: CompiledMember,
    },
}

/// What a class file declares, decoded. Cloned out of the lake rather than
/// borrowed: the walk collects across several classes and several sources, and
/// a `Field` is a string and two enums.
#[derive(Debug, Clone)]
pub(crate) enum CompiledMember {
    Field(jvm::model::Field),
    Method(jvm::model::Method),
    Type {
        fqn: jvm::model::BinaryName,
        kind: jvm::model::TypeKind,
        /// §4.7.6's level, which `class_access` already decoded. `None` is
        /// §8.1.1's local or anonymous class, outside access control.
        access: Option<jvm::model::AccessLevel>,
    },
}

impl CompiledMember {
    pub(crate) fn name(&self) -> &str {
        match self {
            Self::Field(field) => &field.name,
            Self::Method(method) => &method.name,
            Self::Type { fqn, .. } => fqn.simple_name(),
        }
    }

    pub(crate) fn access(&self) -> Option<jvm::model::AccessLevel> {
        match self {
            Self::Field(field) => Some(field.access),
            Self::Method(method) => Some(method.access),
            Self::Type { access, .. } => *access,
        }
    }
}

/// §15.12.1's "class to search", flattened for a type that inherits.
///
/// §8.2 defines a class's members recursively: the ones it declares, then the
/// ones it inherits from its direct superclass — which brings its own inherited
/// members with it — then the same from each direct superinterface. Flattening
/// that is depth first with the superclass before the interfaces.
///
/// The type itself is the first entry, because §8.2 makes its own declarations
/// the nearest ones and a caller wants one list rather than two.
///
/// A type is visited once. §8.1.5 lets two superinterfaces share an ancestor,
/// and a cycle is illegal (§8.1.4) but entirely writable while someone is
/// typing, so one set stops the diamond and the loop together.
pub(crate) fn types_to_search(target: &TypeTarget, query: &Query) -> Vec<TypeTarget> {
    let mut order = Vec::new();
    let mut visited = HashSet::new();
    walk(target, query, &mut order, &mut visited);
    order
}

fn walk(
    target: &TypeTarget,
    query: &Query,
    order: &mut Vec<TypeTarget>,
    visited: &mut HashSet<TypeTarget>,
) {
    if !visited.insert(target.clone()) {
        return;
    }
    order.push(target.clone());

    for supertype in direct_supertypes(target, query) {
        walk(&supertype, query, order, visited);
    }
}

/// What a type names in its `extends` and `implements` clauses, read from
/// whichever half holds it — plus the one supertype nobody writes.
fn direct_supertypes(target: &TypeTarget, query: &Query) -> Vec<TypeTarget> {
    match target {
        TypeTarget::Parsed {
            source,
            declaration,
        } => {
            let Some(file) = query.model_of(source) else {
                return Vec::new();
            };
            let model::Declaration::Type(ty) = &file.declarations[declaration.0] else {
                return Vec::new();
            };

            // §8.1.4 and §9.1.3 write a supertype's name beside the class and
            // not inside it, so `declaring_scope` is where the name is looked
            // up — the body scope would let a member type shadow the clause.
            let written: Vec<_> = ty
                .supertypes()
                .flat_map(|(_, type_ref)| {
                    resolve_type_reference(source, file, type_ref, ty.declaring_scope, query)
                })
                .collect();

            match implicit_superclass(ty) {
                Some(implicit) if ty.superclass.is_none() => written
                    .into_iter()
                    .chain(query.types_named(&jvm::model::BinaryName::new(implicit)))
                    .collect(),
                _ => written,
            }
        }
        TypeTarget::Compiled { source, fqn } => {
            let Some(class) = query.compiled_class(source, fqn) else {
                return Vec::new();
            };

            // JVMS §4.1 puts both clauses in the class file, so a compiled hop
            // needs no resolution at all: the names are already binary.
            class
                .superclass
                .iter()
                .chain(class.interfaces.iter())
                .flat_map(|supertype| query.types_named(supertype))
                .collect()
        }
    }
}

/// The supertype a declaration has without writing one, as §8.1.4 lists them:
///
/// > - The class `Object` has no direct superclass type.
/// > - For a class other than `Object` with a normal class declaration, the
/// >   direct superclass type is `Object`.
/// > - For an enum class E, the direct superclass type is `Enum<E>`.
/// > - For a record class R, the direct superclass type is `Record`.
///
/// with §9.6 adding `java.lang.annotation.Annotation` for an annotation
/// interface. It is not five ways of reaching `Object`: `Enum` declares
/// `name()`, `ordinal()` and `compareTo`, and a walk that jumped straight to
/// `Object` would lose them.
///
/// The interface arm is the one that bends the spec. §9.2 says an interface does
/// *not* inherit from `Object` but implicitly declares members corresponding to
/// its `public` ones, and the difference decides overriding and what counts as a
/// functional interface — neither of which we do. A popup cannot see the
/// difference except for `clone()` and `finalize()`, which are `protected` and
/// should not be offered and are, because `is_compiled_accessible` answers
/// `true` for `Protected` unconditionally. `TODO.md` carries both halves.
///
/// Only the *superclass* is implicit. §8.1.5 leaves a type with no `implements`
/// clause with no superinterfaces at all, so there is nothing to add there.
fn implicit_superclass(ty: &model::TypeDeclaration) -> Option<&'static str> {
    match ty.kind {
        model::TypeKind::Class | model::TypeKind::Interface => Some("java.lang.Object"),
        model::TypeKind::Enum => Some("java.lang.Enum"),
        model::TypeKind::Record => Some("java.lang.Record"),
        model::TypeKind::AnnotationInterface => Some("java.lang.annotation.Annotation"),
    }
}

/// Every member of a type in one namespace, inherited ones included, nearest
/// first.
///
/// Nearest first is what makes the list usable rather than merely complete:
/// §8.3 has a field redeclaration *hide* the one above it and §8.5 says the
/// same of a member type, so the first entry for a name is the one that wins
/// and a caller can drop the rest.
///
/// §6.6.1 is not applied here. §8.2 does exclude a private member from what a
/// subclass inherits, but that is the same question the caller already asks of
/// every member it shows, and asking it in one place beats asking it in two
/// with different answers.
pub(crate) fn members_in_hierarchy(
    target: &TypeTarget,
    namespace: model::Namespace,
    wanted: Wanted<'_>,
    query: &Query,
) -> Vec<Member> {
    types_to_search(target, query)
        .iter()
        .flat_map(|target| members_declared_by(target, namespace, wanted, query))
        .collect()
}

/// The members one type declares, without its supertypes.
fn members_declared_by(
    target: &TypeTarget,
    namespace: model::Namespace,
    wanted: Wanted<'_>,
    query: &Query,
) -> Vec<Member> {
    match target {
        TypeTarget::Parsed {
            source,
            declaration,
        } => {
            let Some(file) = query.model_of(source) else {
                return Vec::new();
            };
            members_of(file, *declaration, namespace, wanted)
                .map(|declaration| Member::Parsed {
                    source: source.clone(),
                    declaration,
                })
                .collect()
        }
        TypeTarget::Compiled { source, fqn } => {
            let Some(class) = query.compiled_class(source, fqn) else {
                return Vec::new();
            };
            let compiled = |declaration| Member::Compiled {
                source: source.clone(),
                owner: fqn.clone(),
                declaration,
            };

            match namespace {
                model::Namespace::Variable => class
                    .fields
                    .iter()
                    .filter(|field| wanted.matches(&field.name))
                    .map(|field| compiled(CompiledMember::Field(field.clone())))
                    .collect(),
                model::Namespace::Method => class
                    .methods
                    .iter()
                    .filter(|method| wanted.matches(&method.name))
                    // JVMS §2.9 names a constructor `<init>` and a class
                    // initializer `<clinit>`. Neither is a member (§8.2 says so
                    // of a constructor outright) and neither is a name anybody
                    // can write, `<` not being an identifier character (§3.8).
                    .filter(|method| !method.name.starts_with('<'))
                    .map(|method| compiled(CompiledMember::Method(method.clone())))
                    .collect(),
                model::Namespace::Type => query
                    .compiled_member_types(fqn)
                    // §13.1 gives a local or anonymous class a digit sequence
                    // after the `$`, and §3.8 keeps a digit out of the first
                    // position of an identifier — so `String$1CharIterator` is
                    // in the image and is not a name anybody can write after a
                    // dot. `Undecided` in `TODO.md` is the same `$1` we cannot
                    // spell from the other direction.
                    .filter(|(_, nested)| {
                        let name = nested.fqn.simple_name();
                        wanted.matches(name)
                            && !name.starts_with(|character: char| character.is_ascii_digit())
                    })
                    .map(|(nested_source, nested)| Member::Compiled {
                        source: nested_source.clone(),
                        owner: fqn.clone(),
                        declaration: CompiledMember::Type {
                            fqn: nested.fqn.clone(),
                            kind: nested.kind,
                            access: nested.access,
                        },
                    })
                    .collect(),
            }
        }
    }
}

/// What a name reaches through a type, hiding applied.
///
/// Only the nearest type to declare the name contributes, which is §8.3 for a
/// field and §8.5 for a member type. Methods are the case the rule cannot
/// state: an inherited method of the same name is overridden (§8.4.8) or
/// overloaded (§8.4.9) rather than hidden, and telling those apart needs
/// §8.4.2's signature with both parameter lists resolved. Nearest wins is the
/// approximation, and it is the right one for navigation — it lands where a
/// reader looking for the declaration would stop.
pub(crate) fn find_member(
    target: &TypeTarget,
    name: &model::Identifier,
    namespace: model::Namespace,
    query: &Query,
) -> Vec<Member> {
    for target in types_to_search(target, query) {
        let found = members_declared_by(&target, namespace, Wanted::Exactly(&name.text), query);
        if !found.is_empty() {
            return found;
        }
    }

    Vec::new()
}
