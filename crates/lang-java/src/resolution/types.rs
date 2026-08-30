//! JLS §6.5.5: what a name means when it is used as a type.
//!
//! The staging is the whole content of §6.4.1 for this namespace, and it is
//! stated once, in `Stage`. `resolve_type_name` filters the chain by one name
//! and completion filters it by a prefix; neither owns the order.

use beans_platform_jvm as jvm;

use crate::accessibility::{Site, is_accessible, is_compiled_accessible};
use crate::model;

use super::methods::members_of;
use crate::query::Query;

use super::Wanted;

use super::candidates::{
    ClassifiedTypeCandidate, InvalidTypeCandidate, ResolutionCandidates, TypeInvalidity,
    TypeResolution, TypeTarget, first_stage_that_answers,
};

/// JLS 26 §6.5.5.1 does not stage this. It asks only that a simple type name
/// occur in the scope of exactly one declaration of a class, interface, or type
/// parameter (§6.3). The staging below is where that "exactly one" comes from:
/// several declarations are usually in scope at once, and §6.4.1 says which of
/// them shadows which. So each stage cites the rule that puts it where it is,
/// and the order is a reading of §6.4.1 rather than a procedure the spec spells
/// out.
pub fn resolve_type_name(
    name: &model::Name,
    source: &jvm::model::Source,
    file: &model::File,
    current_lexical_scope_id: model::LexicalScopeId,
    query: &Query,
) -> TypeResolution {
    resolve_type_candidates(name, source, file, current_lexical_scope_id, query).into_resolution()
}

pub(crate) fn resolve_type_candidates(
    name: &model::Name,
    source: &jvm::model::Source,
    file: &model::File,
    current_lexical_scope_id: model::LexicalScopeId,
    query: &Query,
) -> ResolutionCandidates {
    // §6.5.5.2. A qualified name classifies its prefix as a package or a type
    // first; `resolve_canonical_name` walks that, but only for import names so
    // far.
    let model::Name::Simple(name) = name else {
        return ResolutionCandidates::default();
    };

    let from = Site {
        source,
        file,
        scope: current_lexical_scope_id,
    };

    first_stage_that_answers(
        types_in_scope(&from, query, Wanted::Exactly(&name.text)),
        &from,
        query,
    )
}

/// Which rule put a candidate where it is. Ordering is the whole content of
/// §6.4.1, so it lives here and is read by both the resolver above and the
/// enumeration completion runs over the same chain.
///
/// Lexical scopes are one stage *each* rather than one between them: §6.4.1
/// shadows across the chain as well as across the stages, so an inner
/// declaration has to beat an outer one of the same name outright.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stage {
    /// Type parameters, member types and local types, by distance from the
    /// point. §6.3 for being in scope, §6.4.1 for nearest-first.
    LexicalScope(usize),
    /// §7.5.1.
    ExactImport,
    /// §6.3: the top-level types of this compilation unit's package.
    SamePackage,
    /// §7.3's implicit `java.lang.*`. §7.5.2's written-out on-demand imports
    /// join it here when they are read.
    JavaLang,
}

/// One candidate a stage spells, before scope and §6.6.1 have been asked about
/// it. Classification is deliberately late: it costs two lake lookups, and
/// resolving one name must not pay for the whole package it sits in.
pub(crate) struct InScopeType {
    pub(crate) name: String,
    pub(crate) stage: Stage,
    candidate: Unclassified,
}

enum Unclassified {
    Target(TypeTarget),
    /// §6.5.4.2's walk classifies each segment on the way, because an
    /// inaccessible enclosing type has to propagate onto its member. What it
    /// hands back is already decided.
    Decided(ClassifiedTypeCandidate),
}

impl InScopeType {
    pub(crate) fn classify(self, query: &Query, from: &Site) -> ClassifiedTypeCandidate {
        match self.candidate {
            Unclassified::Target(target) => classify_type_target(target, query, from),
            Unclassified::Decided(candidate) => candidate,
        }
    }
}

/// Every type in scope at `from` that `wanted` admits, in the order §6.4.1 ranks
/// them.
///
/// This is the one statement of that order. `resolve_type_name` filters it by
/// one name; completion filters it by a prefix. Lazily chained, so a name found
/// in a near stage never enumerates a far one.
///
/// Stages 5 and 6 (module imports, suggestions) are not implemented. Their
/// absence means no candidate, never a guess.
pub(crate) fn types_in_scope<'a>(
    from: &'a Site<'a>,
    query: &'a Query<'a>,
    wanted: Wanted<'a>,
) -> impl Iterator<Item = InScopeType> + 'a {
    lexical_scopes(from, wanted)
        .chain(exact_imports(from, query, wanted))
        .chain(same_package(from, query, wanted))
        .chain(java_lang(from, query, wanted))
}

/// Stage 1. §6.3 puts type parameters, member types and local types in scope;
/// §6.4.1 makes the nearest declaration shadow the rest, which is why each
/// scope in the chain is its own stage rather than all of them being one.
fn lexical_scopes<'a>(
    from: &'a Site<'a>,
    wanted: Wanted<'a>,
) -> impl Iterator<Item = InScopeType> + 'a {
    from.file
        .iter_scope_chain(from.scope)
        .enumerate()
        .flat_map(move |(depth, (_, scope))| {
            scope
                .declarations
                .iter()
                .copied()
                .filter_map(move |declaration_id| {
                    let declaration = from.file.declarations.get(declaration_id.0)?;
                    let name = match declaration {
                        model::Declaration::Type(declaration) => declaration.name.as_ref(),
                        model::Declaration::TypeParameter(declaration) => Some(&declaration.name),
                        _ => None,
                    }?;
                    if !wanted.matches(&name.text) {
                        return None;
                    }

                    Some(InScopeType {
                        name: name.text.clone(),
                        stage: Stage::LexicalScope(depth),
                        // Valid outright rather than classified: the chain only
                        // ever reaches declarations of this file that enclose
                        // the point, so both ends of §6.6.1's question are the
                        // same compilation unit and scope membership is a
                        // question about somewhere else.
                        candidate: Unclassified::Decided(ClassifiedTypeCandidate::Valid(
                            TypeTarget::Parsed {
                                source: from.source.clone(),
                                declaration: declaration_id,
                            },
                        )),
                    })
                })
        })
}

/// Stage 2. §7.5.1 rejects an inaccessible import, and javac keeps that failure
/// as recovery evidence without letting it hide an answer from a later stage —
/// which is what `first_stage_that_answers` does with the invalid half.
fn exact_imports<'a>(
    from: &'a Site<'a>,
    query: &'a Query<'a>,
    wanted: Wanted<'a>,
) -> impl Iterator<Item = InScopeType> + 'a {
    from.file
        .imports
        .iter()
        .filter(|import| import.kind == model::ImportKind::Type)
        .filter_map(move |import| {
            let name = import.name.segments().last()?.text.clone();
            // Before the walk, not after: §6.5.4.2 probes the lake once per
            // segment, and an import nobody asked about should cost nothing.
            if !wanted.matches(&name) {
                return None;
            }
            Some((name, resolve_canonical_name(&import.name, from, query)))
        })
        .flat_map(|(name, candidates)| {
            candidates.into_iter().map(move |candidate| InScopeType {
                name: name.clone(),
                stage: Stage::ExactImport,
                candidate: Unclassified::Decided(candidate),
            })
        })
}

/// Stage 3. §6.3: the top-level types of this compilation unit's package.
///
/// A member type is dropped rather than offered: §6.5.5.1 puts one in scope by
/// its simple name inside its enclosing type or through an import, never by
/// sharing a package. The lake carries the package and nothing else about where
/// a type sits, so §13.1's `$` is what tells them apart.
fn same_package<'a>(
    from: &'a Site<'a>,
    query: &'a Query<'a>,
    wanted: Wanted<'a>,
) -> impl Iterator<Item = InScopeType> + 'a {
    let package = from
        .file
        .package
        .as_ref()
        .map(model::Name::dotted)
        .unwrap_or_default();

    types_of_package(package, Stage::SamePackage, query, wanted)
}

/// Deferred so that building the chain costs nothing: the package traversal
/// only runs if something pulls from this stage, which only happens when every
/// nearer stage came up empty.
fn types_of_package<'a>(
    package: String,
    stage: Stage,
    query: &'a Query<'a>,
    wanted: Wanted<'a>,
) -> impl Iterator<Item = InScopeType> + 'a {
    std::iter::once(package).flat_map(move |package| {
        // One name is the binary name the lake was told at projection time, so
        // it is a lookup rather than a walk over the package. A prefix has no
        // such key and traverses.
        let found: Vec<(String, TypeTarget)> = match wanted.keyed() {
            Some(name) => query
                .types_named(&jvm::model::BinaryName::in_package(&package, name))
                .into_iter()
                .map(|target| (name.to_string(), target))
                .collect(),
            None => query
                .top_level_types_in_package(&package)
                .filter(|(name, _)| wanted.matches(name))
                .collect(),
        };

        found.into_iter().map(move |(name, target)| InScopeType {
            name,
            stage,
            candidate: Unclassified::Target(target),
        })
    })
}

/// Stage 4. §7.3 treats every compilation unit as if `import java.lang.*;` stood
/// after its package declaration, so this is stage 3 with the package fixed.
///
/// §7.3 imports the `public` types and a runtime image is full of the others.
/// Keeping `java.lang.Shutdown` off the path `java.lang.String` takes is
/// §6.6.1's doing, applied when these are classified rather than here.
fn java_lang<'a>(
    _from: &'a Site<'a>,
    query: &'a Query<'a>,
    wanted: Wanted<'a>,
) -> impl Iterator<Item = InScopeType> + 'a {
    types_of_package("java.lang".to_string(), Stage::JavaLang, query, wanted)
}

/// One stage of the chain, asked about one name. Every case that used to call a
/// stage function directly goes through here, so a stage is still testable in
/// isolation now that the pipeline is shared.
#[cfg(test)]
fn one_stage(
    candidates: impl Iterator<Item = InScopeType>,
    from: &Site,
    query: &Query,
) -> ResolutionCandidates {
    first_stage_that_answers(candidates, from, query)
}

#[cfg(test)]
fn candidates_from_exact_imports(
    name: &model::Identifier,
    source: &jvm::model::Source,
    file: &model::File,
    query: &Query,
) -> ResolutionCandidates {
    let from = Site {
        source,
        file,
        scope: model::LexicalScopeId(0),
    };
    one_stage(
        exact_imports(&from, query, Wanted::Exactly(&name.text)),
        &from,
        query,
    )
}

#[cfg(test)]
fn candidates_from_same_package(
    name: &model::Identifier,
    from: &Site,
    query: &Query,
) -> ResolutionCandidates {
    one_stage(
        same_package(from, query, Wanted::Exactly(&name.text)),
        from,
        query,
    )
}

#[cfg(test)]
fn resolve_type_from_lexical_scopes(
    name: &model::Identifier,
    source: &jvm::model::Source,
    file: &model::File,
    current_lexical_scope_id: model::LexicalScopeId,
) -> TypeResolution {
    let from = Site {
        source,
        file,
        scope: current_lexical_scope_id,
    };
    // Stage 1 decides its own candidates, so the query is never consulted.
    let jvm = jvm::Platform::new();
    let java = crate::Language::new();
    let query = Query::new(
        jvm::query::Query::new(&jvm, jvm::query::ScopeQuery::unscoped(), Default::default()),
        &java,
    );
    one_stage(
        lexical_scopes(&from, Wanted::Exactly(&name.text)),
        &from,
        &query,
    )
    .into_resolution()
}

#[cfg(test)]
fn resolve_type_from_exact_imports(
    name: &model::Identifier,
    source: &jvm::model::Source,
    file: &model::File,
    query: &Query,
) -> TypeResolution {
    let from = Site {
        source,
        file,
        scope: model::LexicalScopeId(0),
    };
    candidates_from_exact_imports(name, source, file, query).into_resolution()
}

#[cfg(test)]
fn resolve_from_same_package(
    name: &model::Identifier,
    source: &jvm::model::Source,
    file: &model::File,
    query: &Query,
) -> TypeResolution {
    let from = Site {
        source,
        file,
        scope: model::LexicalScopeId(0),
    };
    candidates_from_same_package(name, &from, query).into_resolution()
}

/// A dotted name hides where the package stops and the type begins. JLS 26
/// §6.5.4.2 answers it with one question asked left to right: does whatever `Q`
/// denotes so far have a member class or interface named `Id`? It starts on
/// the unnamed package (§6.5.4.1) and switches to a type path the first time an
/// observable declaration answers. Segments join with `.` before the switch
/// and with `$` after it (§13.1).
///
/// Accessibility is checked later by §6.5.5.2, so an observable but
/// inaccessible declaration still commits the type path and then becomes
/// invalid. A declaration outside the host compilation described by §7.3 does
/// not alter the semantic package path, but Beans carries it on a parallel
/// invalid path as recovery evidence. Nothing is guessed and no prefix is tried
/// twice on the same path.
///
/// Package existence is never probed. §6.5.2 defers it ("A later step
/// determines whether or not a package of that name actually exists"), and
/// §6.5.4.2 only ever falls back to reclassifying as a package name, so ending
/// the walk in package mode is unresolved and nothing more. §6.5.5.2 is what
/// makes that final: a type name `Q.Id` is a compile-time error unless `Id`
/// names exactly one accessible member of `Q`, and its "more than one" clause is
/// where case 8's ambiguity comes from. Note §6.5.4.2 asks only about a member
/// class or interface; the field and method arms belong to ambiguous names
/// (§6.5.2), and a type name is not an ambiguous name.
///
/// 1. The spec's own worked example.
///
/// ```text
/// lake: java.util.Date
///
/// java   probe java            miss  -> package java
/// util   probe java.util       miss  -> package java.util
/// Date   probe java.util.Date  hit   -> type
/// ```
///
/// 2. A nested type in a Java file. Projection registers the top-level class
///    only, so the member step walks the model and no `$` is ever spelled.
///
/// ```text
/// lake: p.Outer            (Outer.java declares Inner inside it)
///
/// p      probe p             miss  -> package p
/// Outer  probe p.Outer       hit   -> type, Parsed arm
/// Inner  members_of(Inner)   hit   -> type, Parsed arm
/// ```
///
/// 3. The same spelling against a class file. Nesting is flat in the lake, and
///    the mode change is where the dot becomes a dollar.
///
/// ```text
/// lake: java.util.Map, java.util.Map$Entry
///
/// java   probe java                 miss  -> package java
/// util   probe java.util            miss  -> package java.util
/// Map    probe java.util.Map        hit   -> type, Compiled arm
/// Entry  probe java.util.Map$Entry  hit   -> type, Compiled arm
/// ```
///
/// 4. Deeper nesting is the same step repeated.
///
/// ```text
/// lake: p.A, p.A$B, p.A$B$C
///
/// p  probe p        miss  -> package p
/// A  probe p.A      hit   -> type
/// B  probe p.A$B    hit   -> type
/// C  probe p.A$B$C  hit   -> type
/// ```
///
/// 5. The unnamed package, where the empty prefix joins without a leading dot.
///
/// ```text
/// lake: Outer, Outer$Inner
///
/// Outer  probe Outer        hit  -> type
/// Inner  probe Outer$Inner  hit  -> type
/// ```
///
/// 6. A name nothing declares reaches the end still in package mode.
///
/// ```text
/// lake: (nothing under com)
///
/// com      probe com                  miss  -> package com
/// example  probe com.example          miss  -> package com.example
/// Missing  probe com.example.Missing  miss  -> package com.example.Missing
///                                           -> unresolved
/// ```
///
/// 7. A type and a package sharing a name. Once `p.Outer` reclassifies as a
///    type, §6.5.4.2 offers only a member type, so the top-level
///    `p.Outer.Inner` is unreachable by that spelling even though the lake
///    holds exactly that binary name. §7.1 makes the collision a compile-time
///    error, so only a broken program gets here; the walk still has to answer,
///    and it answers by the same rule as everything else.
///
/// ```text
/// lake: p.Outer        (a class)
///       p.Outer.Inner  (a top-level class in package p.Outer)
///
/// p      probe p             miss  -> package p
/// Outer  probe p.Outer       hit   -> type
/// Inner  member type Inner?  no    -> unresolved
/// ```
///
/// 8. Two containers declaring one name. The probe answers with a set, so the
///    walk fans out over it and any ambiguity survives to the end instead of
///    being settled on the way.
///
/// ```text
/// lake: p.B in app/src, p.B in lib/src   (scope holds both trees)
///
/// p  probe p    miss    -> package p
/// B  probe p.B  2 hits  -> type {app, lib}  -> ambiguous
/// ```
fn resolve_canonical_name(name: &model::Name, from: &Site, query: &Query) -> ResolutionCandidates {
    let mut paths = vec![CanonicalPath::Package(String::new())];

    for segment in name.segments() {
        let mut next = Vec::new();
        for path in paths {
            match path {
                CanonicalPath::Package(prefix) => {
                    let name = jvm::model::BinaryName::in_package(&prefix, &segment.text);
                    let candidates = classify_types_named(query, &name, from);
                    if !candidates.commits_type_path() {
                        next.push(CanonicalPath::Package(name.as_str().to_owned()));
                    }
                    next.extend(candidates.into_iter().map(CanonicalPath::Type));
                }
                CanonicalPath::Type(parent) => {
                    for child in member_types(parent.target(), segment, query) {
                        let child = classify_type_target(child, query, from);
                        next.push(CanonicalPath::Type(parent.clone().propagate(child)));
                    }
                }
            }
        }
        paths = next;
    }

    paths
        .into_iter()
        .filter_map(|path| match path {
            CanonicalPath::Package(_) => None,
            CanonicalPath::Type(candidate) => Some(candidate),
        })
        .collect()
}

#[derive(Clone)]
enum CanonicalPath {
    /// The dotted package name walked so far; empty is the unnamed package.
    Package(String),
    /// One classified path through a type and any subsequent member types.
    Type(ClassifiedTypeCandidate),
}

/// §6.5.4.2's TypeName branch: member types before scope and access are
/// composed. A file this vertical parsed holds its members in the model;
/// anything else holds them in the lake under a nested binary name.
fn member_types(target: &TypeTarget, name: &model::Identifier, query: &Query) -> Vec<TypeTarget> {
    match target {
        TypeTarget::Parsed {
            source,
            declaration,
        } => {
            let Some(file) = query.model_of(source) else {
                return Vec::new();
            };
            members_of(
                file,
                *declaration,
                model::Namespace::Type,
                Wanted::Exactly(&name.text),
            )
            .map(|declaration| TypeTarget::Parsed {
                source: source.clone(),
                declaration,
            })
            .collect()
        }
        TypeTarget::Compiled { fqn, .. } => query.types_named(&fqn.nested(&name.text)),
    }
}

fn classify_types_named(
    query: &Query,
    fqn: &jvm::model::BinaryName,
    from: &Site,
) -> ResolutionCandidates {
    query
        .types_named(fqn)
        .into_iter()
        .map(|target| classify_type_target(target, query, from))
        .collect()
}

fn classify_type_target(target: TypeTarget, query: &Query, from: &Site) -> ClassifiedTypeCandidate {
    if query.scope_membership(&target) == jvm::query::ScopeMembership::OutsideScope {
        return ClassifiedTypeCandidate::Invalid(InvalidTypeCandidate::rejected(
            target,
            TypeInvalidity::OutsideScope,
        ));
    }

    if type_target_is_accessible(&target, query, from) {
        ClassifiedTypeCandidate::Valid(target)
    } else {
        ClassifiedTypeCandidate::Invalid(InvalidTypeCandidate::rejected(
            target,
            TypeInvalidity::Inaccessible,
        ))
    }
}

fn type_target_is_accessible(target: &TypeTarget, query: &Query, from: &Site) -> bool {
    match target {
        TypeTarget::Parsed {
            source,
            declaration,
        } => {
            let Some(file) = query.model_of(source) else {
                return true;
            };
            let model::Declaration::Type(declaration) = &file.declarations[declaration.0] else {
                return true;
            };
            let declared = Site {
                source,
                file,
                scope: declaration.declaring_scope,
            };

            is_accessible(declaration.access, &declared, from)
        }
        // A binary name carries its own package (§13.1), so the declaring end of
        // §6.6.1 needs nothing the target does not already say but the level.
        TypeTarget::Compiled { source, fqn } => {
            is_compiled_accessible(query.class_access(source, fqn), fqn.package(), from)
        }
    }
}

/// A syntactic type annotation resolved to the type it names.
///
/// Both arms come back. A caller that needs a span to jump to drops the
/// compiled one on the way out, but a caller walking a hierarchy must not:
/// `Object`, `Enum` and `Record` are class files in every Java program, so
/// filtering here would end every walk one hop early.
pub(super) fn resolve_type_reference(
    source: &jvm::model::Source,
    file: &model::File,
    type_ref: &model::TypeRef,
    scope: model::LexicalScopeId,
    query: &Query,
) -> Vec<TypeTarget> {
    // A primitive and `void` name no declaration (§4.2, §8.4.5); an array
    // resolves through its component type (§10.1).
    let Some(name) = type_ref.ty.named() else {
        return Vec::new();
    };

    match resolve_type_name(name, source, file, scope, query) {
        TypeResolution::Resolved(target) => vec![target],
        TypeResolution::Ambiguous(targets) => targets,
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests;
