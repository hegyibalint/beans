//! Resolving an occurrence to what it denotes.
//!
//! JLS §6.5 splits this by what a name is used *as*, and so does this module.
//! §6.1's three namespaces turn out to have three different sets of rules —
//! types stage across imports, packages and the lake; a variable never leaves
//! the scope chain; a method ends in §15.12.2's applicability — so they share a
//! `Site`, a `Wanted` and the shape of the fold rather than the rules.
//!
//! What stays here is the dispatch: deciding which of the three is being asked,
//! which is §6.5.2's job and belongs to none of them.

pub(crate) mod candidates;
pub(crate) mod hierarchy;
pub(crate) mod methods;
pub(crate) mod types;
pub(crate) mod variables;

use beans_core::language::NavigationTarget;
use beans_core::model::Offset;
use beans_platform_jvm as jvm;

use crate::{model, query::Query};

pub(crate) use candidates::{ResolutionCandidates, TypeInvalidity, first_stage_that_answers};
pub use candidates::{TypeResolution, TypeTarget};
pub(crate) use hierarchy::{CompiledMember, Member, find_member, members_in_hierarchy};
pub(crate) use methods::{InScopeMethod, methods_in_scope};
pub(crate) use types::{InScopeType, resolve_type_candidates, resolve_type_name, types_in_scope};
pub(crate) use variables::{InScopeVariable, resolve_variable_name, variables_in_scope};

use types::resolve_type_reference;

/// What the chain is being asked for.
///
/// The two questions cost different things and a stage can tell them apart: one
/// name is a keyed lookup into the lake, a prefix is a traversal of a package.
/// Stating it here is what lets resolution and completion share one enumeration
/// without resolution paying completion's price — filtering the answers instead
/// would build a candidate for all 145 types of `java.lang` to keep one.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Wanted<'a> {
    Exactly(&'a str),
    StartingWith(&'a str),
}

impl Wanted<'_> {
    pub(crate) fn matches(&self, name: &str) -> bool {
        match self {
            Self::Exactly(wanted) => name == *wanted,
            Self::StartingWith(prefix) => name.starts_with(prefix),
        }
    }

    /// The one name a stage may look up directly, if that is what was asked.
    pub(super) fn keyed(&self) -> Option<&str> {
        match self {
            Self::Exactly(wanted) => Some(wanted),
            Self::StartingWith(_) => None,
        }
    }
}

/// Resolves the occurrence at `offset` to the declarations it refers to.
/// This is the go-to-declaration entry point.
pub fn resolve_occurrence_at(
    source: &jvm::model::Source,
    file: &model::File,
    offset: Offset,
    query: &Query,
) -> Vec<NavigationTarget<jvm::model::Source>> {
    let Some((_, entity)) = file.position_index.tightest_containing(offset) else {
        return Vec::new();
    };

    // Navigation is the one caller that cannot use half of what resolution
    // finds. A `TypeTarget::Compiled` and a `Member::Compiled` are answers — the
    // caret really does denote `java.lang.Object` — but a class file has no
    // source to open, so both are dropped here rather than earlier.
    match entity {
        model::EntityId::Declaration(declaration) => declaration_target(source, declaration, query)
            .into_iter()
            .collect(),
        model::EntityId::TypeRef(owner) => {
            let declaration = &file.declarations[owner.0];
            let Some(type_ref) = declaration.type_ref() else {
                return Vec::new();
            };
            navigable_types(
                resolve_type_reference(
                    source,
                    file,
                    type_ref,
                    declaration.declaring_scope(),
                    query,
                ),
                query,
            )
        }
        // §8.1.4 and §9.1.3 want the name in an `extends` or `implements`
        // clause resolved where it was written, so the scope is the one the
        // type is declared in and not its own body.
        model::EntityId::Supertype(owner, supertype) => {
            let model::Declaration::Type(declaration) = &file.declarations[owner.0] else {
                return Vec::new();
            };
            let Some(type_ref) = declaration.supertype(supertype) else {
                return Vec::new();
            };
            navigable_types(
                resolve_type_reference(source, file, type_ref, declaration.declaring_scope, query),
                query,
            )
        }
        model::EntityId::BodyNode(body, node) => {
            let model::BodyNodeKind::Expression(_) = &file.bodies[body.0].node(node).kind else {
                return Vec::new();
            };
            resolve_expression(source, file, body, node, query)
                .into_iter()
                .filter_map(|member| match member {
                    Member::Parsed {
                        source,
                        declaration,
                    } => declaration_target(&source, declaration, query),
                    Member::Compiled { .. } => None,
                })
                .collect()
        }
        model::EntityId::Scope(..) | model::EntityId::Import(..) => Vec::new(),
    }
}

fn navigable_types(
    targets: Vec<TypeTarget>,
    query: &Query,
) -> Vec<NavigationTarget<jvm::model::Source>> {
    targets
        .into_iter()
        .filter_map(|target| match target {
            TypeTarget::Parsed {
                source,
                declaration,
            } => declaration_target(&source, declaration, query),
            TypeTarget::Compiled { .. } => None,
        })
        .collect()
}

fn declaration_target(
    source: &jvm::model::Source,
    declaration: model::DeclarationId,
    query: &Query,
) -> Option<NavigationTarget<jvm::model::Source>> {
    let file = query.model_of(source)?;
    Some(NavigationTarget {
        source: source.clone(),
        span: file.declarations[declaration.0].name_span()?,
    })
}

pub(crate) fn resolve_expression(
    source: &jvm::model::Source,
    file: &model::File,
    body_id: model::BodyId,
    expression_id: model::BodyNodeId,
    query: &Query,
) -> Vec<Member> {
    let body = &file.bodies[body_id.0];
    let scope = body.node(expression_id).scope;
    let Some(expression) = body.expression(expression_id) else {
        return Vec::new();
    };
    match expression {
        model::Expression::NameRef { name } => resolve_variable_name(file, name, scope)
            .into_iter()
            .map(|declaration| Member::Parsed {
                source: source.clone(),
                declaration,
            })
            .collect(),
        model::Expression::This => file
            .enclosing_type_declaration(scope)
            .map(|declaration| {
                vec![Member::Parsed {
                    source: source.clone(),
                    declaration,
                }]
            })
            .unwrap_or_default(),
        model::Expression::FieldAccess { receiver, name } => {
            let Some(class) = resolve_receiver_class(source, file, body_id, *receiver, query)
            else {
                return Vec::new();
            };
            find_member(&class, name, model::Namespace::Variable, query)
        }
        model::Expression::MethodCall { receiver, name, .. } => {
            let Some(class) = receiver_or_this(source, file, body_id, *receiver, scope, query)
            else {
                return Vec::new();
            };
            find_member(&class, name, model::Namespace::Method, query)
        }
        model::Expression::ObjectCreation { ty, .. } => {
            resolve_type_reference(source, file, ty, scope, query)
                .into_iter()
                .filter_map(|target| match target {
                    TypeTarget::Parsed {
                        source,
                        declaration,
                    } => Some(Member::Parsed {
                        source,
                        declaration,
                    }),
                    // §15.9 names a class, and a compiled one has a name and no
                    // declaration to point at. What it really denotes is a
                    // constructor (§8.8), which we do not model at all.
                    TypeTarget::Compiled { .. } => None,
                })
                .collect()
        }
        model::Expression::Assign { .. } | model::Expression::Literal => Vec::new(),
    }
}

/// §15.12.1: a call with no receiver searches the class whose body encloses it,
/// which is the implicit `this`.
fn receiver_or_this(
    source: &jvm::model::Source,
    file: &model::File,
    body_id: model::BodyId,
    receiver: Option<model::BodyNodeId>,
    scope: model::LexicalScopeId,
    query: &Query,
) -> Option<TypeTarget> {
    match receiver {
        Some(receiver) => resolve_receiver_class(source, file, body_id, receiver, query),
        None => Some(TypeTarget::Parsed {
            source: source.clone(),
            declaration: file.enclosing_type_declaration(scope)?,
        }),
    }
}

/// The type through which member lookup for `expression` runs: the declared
/// type of the expression, or the type itself for static access (`Bar.asd`).
fn resolve_receiver_class(
    source: &jvm::model::Source,
    file: &model::File,
    body_id: model::BodyId,
    expression_id: model::BodyNodeId,
    query: &Query,
) -> Option<TypeTarget> {
    let body = &file.bodies[body_id.0];
    let scope = body.node(expression_id).scope;
    let expression = body.expression(expression_id)?;
    match expression {
        model::Expression::This => Some(TypeTarget::Parsed {
            source: source.clone(),
            declaration: file.enclosing_type_declaration(scope)?,
        }),
        model::Expression::NameRef { name } => {
            resolve_name_as_class(name, source, file, scope, query)
        }
        model::Expression::FieldAccess { receiver, name } => {
            let class = resolve_receiver_class(source, file, body_id, *receiver, query)?;
            let member = find_member(&class, name, model::Namespace::Variable, query)
                .into_iter()
                .next()?;
            type_of_member(&member, query)
        }
        model::Expression::MethodCall { receiver, name, .. } => {
            let class = receiver_or_this(source, file, body_id, *receiver, scope, query)?;
            let member = find_member(&class, name, model::Namespace::Method, query)
                .into_iter()
                .next()?;
            type_of_member(&member, query)
        }
        model::Expression::ObjectCreation { ty, .. } => {
            resolve_type_reference(source, file, ty, scope, query)
                .into_iter()
                .next()
        }
        model::Expression::Assign { .. } | model::Expression::Literal => None,
    }
}

/// The type a member's own type denotes, which is what the next `.` searches.
///
/// Two halves because a member has two homes. A parsed one carries a `TypeRef`,
/// a name as written, and §6.5.5 resolves it where it was written — in the
/// member's file and not the receiver's, which is the whole reason `Member`
/// carries a source of its own. A compiled one carries a descriptor the lake
/// already decoded, so JVMS §4.3.2 has done the resolving and a binary name is
/// left.
///
/// A primitive, a `void` return and a name nothing answers all give `None`.
/// §4.2 leaves a primitive with no members, so a chain ends there rather than
/// continuing through something it is not.
fn type_of_member(member: &Member, query: &Query) -> Option<TypeTarget> {
    match member {
        Member::Parsed {
            source,
            declaration,
        } => {
            let file = query.model_of(source)?;
            let declaration = &file.declarations[declaration.0];
            resolve_type_reference(
                source,
                file,
                declaration.type_ref()?,
                declaration.declaring_scope(),
                query,
            )
            .into_iter()
            .next()
        }
        Member::Compiled { declaration, .. } => {
            let fqn = match declaration {
                CompiledMember::Field(field) => named_jvm_type(&field.jvm_type)?,
                CompiledMember::Method(method) => match &method.return_type {
                    jvm::model::ReturnType::Value(ty) => named_jvm_type(ty)?,
                    jvm::model::ReturnType::Void => return None,
                },
                CompiledMember::Type { fqn, .. } => fqn,
            };
            query.types_named(fqn).into_iter().next()
        }
    }
}

/// The class a JVM type names, if it names one. An array resolves through its
/// component type the way §10.1 says a source one does, and §4.2 leaves a
/// primitive naming nothing.
fn named_jvm_type(ty: &jvm::model::Type) -> Option<&jvm::model::BinaryName> {
    match ty {
        jvm::model::Type::Class(fqn) => Some(fqn),
        jvm::model::Type::Array(component) => named_jvm_type(component),
        jvm::model::Type::Primitive(_) => None,
    }
}

/// §6.5.2.1 for one segment: a variable if the scope chain spells the name, and
/// a type otherwise, which is what makes `Bar.asd` reach a static member.
fn resolve_name_as_class(
    name: &model::Identifier,
    source: &jvm::model::Source,
    file: &model::File,
    scope: model::LexicalScopeId,
    query: &Query,
) -> Option<TypeTarget> {
    if let Some(variable) = resolve_variable_name(file, name, scope).into_iter().next() {
        return type_of_member(
            &Member::Parsed {
                source: source.clone(),
                declaration: variable,
            },
            query,
        );
    }

    match resolve_type_name(
        &model::Name::Simple(name.clone()),
        source,
        file,
        scope,
        query,
    ) {
        TypeResolution::Resolved(target) => Some(target),
        _ => None,
    }
}

/// The type a receiver denotes, read from how it is written rather than from
/// an expression the parser produced.
///
/// `resolve_receiver_class` answers the same question from a `BodyNodeId`. A
/// caret at `a.` has no such node — a trailing dot recovers into an `ERROR`, or
/// worse, and TODO.md records how much worse — so completion has only the
/// spelling to go on. §6.5.2's *AmbiguousName* is exactly a chain of
/// identifiers waiting to be classified, and §6.5.2.1 classifies it left to
/// right: the first segment names a variable or a type, and every later one is
/// a field of what came before.
pub(crate) fn resolve_receiver_name(
    segments: &[model::Identifier],
    source: &jvm::model::Source,
    file: &model::File,
    scope: model::LexicalScopeId,
    query: &Query,
) -> Option<TypeTarget> {
    let (first, rest) = segments.split_first()?;

    // §15.8.3: `this` denotes the instance whose body the caret sits in.
    let mut current = if first.text == "this" {
        TypeTarget::Parsed {
            source: source.clone(),
            declaration: file.enclosing_type_declaration(scope)?,
        }
    } else {
        resolve_name_as_class(first, source, file, scope, query)?
    };

    for segment in rest {
        let member = find_member(&current, segment, model::Namespace::Variable, query)
            .into_iter()
            .next()?;
        current = type_of_member(&member, query)?;
    }

    Some(current)
}
