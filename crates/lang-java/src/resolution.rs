use beans_core::language::NavigationTarget;
use beans_core::model::Offset;
use beans_platform_jvm as jvm;

use crate::{
    accessibility::{Site, is_accessible, is_compiled_type_accessible},
    model,
    query::Query,
};

/// The main result of resolving a type name.
///
/// The important thing is that we separate and supply candidates where we can.
/// This allows us to make better decisions and actions when we have multiple candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeResolution {
    /// We couldn't find anything that resembles the type in need.
    /// As a consequence, we cannot supply any supplimental information
    Unresolved {
        invalid_candidates: Vec<InvalidTypeCandidate>,
    },
    /// We had a successful resolution, with a single candidate.
    /// Our happy path.
    Resolved(TypeTarget),
    /// We had a _too_ successful resolution, and we found more than one candidate.
    /// We store all candidates, so we can use this information further down the line.
    Ambiguous(Vec<TypeTarget>),
}

impl TypeResolution {
    pub(crate) fn has_invalidity(&self, reason: TypeInvalidity) -> bool {
        let Self::Unresolved { invalid_candidates } = self else {
            return false;
        };

        invalid_candidates
            .iter()
            .any(|candidate| candidate.has_invalidity(reason))
    }
}

/// Resolution needs to point out what type we resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeTarget {
    /// A file this vertical parsed, so the whole model is in hand and the
    /// declaration is an index into it.
    Parsed {
        source: jvm::model::Source,
        declaration: model::DeclarationId,
    },
    /// Anything but a Java source file, where the lake holds a binary name and
    /// nothing a declaration id could point at.
    Compiled {
        source: jvm::model::Source,
        fqn: jvm::model::BinaryName,
    },
}

impl TypeTarget {
    pub(crate) fn source(&self) -> &jvm::model::Source {
        match self {
            Self::Parsed { source, .. } | Self::Compiled { source, .. } => source,
        }
    }
}

/// Resolution might find candidates, however, that doesn't mean that those candidates are valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum TypeInvalidity {
    /// The candidate we found is not in the scope of this module.
    /// Think of this as a `main` module using a class from `test`; normally this direction is set up to be impossible.
    OutsideScope,
    /// The candidate we found is in scope, but its access modifiers prohibit us from using it.
    /// Think of using a private class from another package; resolution sees the class, but it understands that
    Inaccessible,
}

/// Represents a resolution candidate that was rejected for some reason
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InvalidTypeCandidate {
    target: TypeTarget,
    reasons: Vec<TypeInvalidity>,
}

impl InvalidTypeCandidate {
    pub(crate) fn target(&self) -> &TypeTarget {
        &self.target
    }

    pub(crate) fn invalidities(&self) -> &[TypeInvalidity] {
        &self.reasons
    }

    pub(crate) fn has_invalidity(&self, reason: TypeInvalidity) -> bool {
        self.invalidities().contains(&reason)
    }

    fn add_reason(&mut self, reason: TypeInvalidity) {
        if !self.has_invalidity(reason) {
            self.reasons.push(reason);
            self.reasons.sort_unstable();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ClassifiedTypeCandidate {
    Valid(TypeTarget),
    Invalid(InvalidTypeCandidate),
}

impl ClassifiedTypeCandidate {
    fn target(&self) -> &TypeTarget {
        match self {
            Self::Valid(target) => target,
            Self::Invalid(candidate) => candidate.target(),
        }
    }

    fn propagate(self, child: ClassifiedTypeCandidate) -> ClassifiedTypeCandidate {
        let Self::Invalid(parent) = self else {
            return child;
        };

        let mut child = match child {
            Self::Valid(target) => InvalidTypeCandidate {
                target,
                reasons: Vec::new(),
            },
            Self::Invalid(candidate) => candidate,
        };
        for reason in parent.reasons {
            child.add_reason(reason);
        }
        Self::Invalid(child)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ResolutionCandidates {
    valid: Vec<TypeTarget>,
    invalid: Vec<InvalidTypeCandidate>,
}

impl ResolutionCandidates {
    fn push(&mut self, candidate: ClassifiedTypeCandidate) {
        match candidate {
            ClassifiedTypeCandidate::Valid(target) => {
                if !self.valid.contains(&target) {
                    self.valid.push(target);
                }
            }
            ClassifiedTypeCandidate::Invalid(candidate) => {
                if !self.invalid.contains(&candidate) {
                    self.invalid.push(candidate);
                }
            }
        }
    }

    fn into_resolution(self) -> TypeResolution {
        classify_candidates(self.valid, self.invalid)
    }

    fn commits_type_path(&self) -> bool {
        self.has_valid()
            || self
                .invalid
                .iter()
                .any(|candidate| candidate.has_invalidity(TypeInvalidity::Inaccessible))
    }

    pub(crate) fn has_valid(&self) -> bool {
        !self.valid.is_empty()
    }

    /// The candidates that survived scope and §6.6.1. Completion offers these
    /// and never the invalid ones: an invalid candidate is evidence for a
    /// diagnostic, not a suggestion.
    pub(crate) fn valid(&self) -> &[TypeTarget] {
        &self.valid
    }

    #[cfg(test)]
    fn has_invalidity(&self, reason: TypeInvalidity) -> bool {
        self.invalid
            .iter()
            .any(|candidate| candidate.has_invalidity(reason))
    }
}

impl FromIterator<ClassifiedTypeCandidate> for ResolutionCandidates {
    fn from_iter<T: IntoIterator<Item = ClassifiedTypeCandidate>>(iter: T) -> Self {
        let mut candidates = Self::default();
        for candidate in iter {
            candidates.push(candidate);
        }
        candidates
    }
}

impl IntoIterator for ResolutionCandidates {
    type Item = ClassifiedTypeCandidate;
    type IntoIter = std::vec::IntoIter<ClassifiedTypeCandidate>;

    fn into_iter(self) -> Self::IntoIter {
        self.valid
            .into_iter()
            .map(ClassifiedTypeCandidate::Valid)
            .chain(
                self.invalid
                    .into_iter()
                    .map(ClassifiedTypeCandidate::Invalid),
            )
            .collect::<Vec<_>>()
            .into_iter()
    }
}

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
    fn matches(&self, name: &str) -> bool {
        match self {
            Self::Exactly(wanted) => name == *wanted,
            Self::StartingWith(prefix) => name.starts_with(prefix),
        }
    }

    /// The one name a stage may look up directly, if that is what was asked.
    fn keyed(&self) -> Option<&str> {
        match self {
            Self::Exactly(wanted) => Some(wanted),
            Self::StartingWith(_) => None,
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

/// §6.4.1 as a fold: the first stage to produce a valid candidate answers, and
/// the invalid ones passed over on the way are kept.
///
/// Keeping them is what javac does and what recovery needs — an inaccessible
/// import is evidence worth reporting, and it must not hide an accessible
/// answer from a later stage. It is also why this cannot be `find`.
pub(crate) fn first_stage_that_answers(
    candidates: impl Iterator<Item = InScopeType>,
    from: &Site,
    query: &Query,
) -> ResolutionCandidates {
    let mut answered = ResolutionCandidates::default();
    let mut stage = None;

    for candidate in candidates {
        if stage != Some(candidate.stage) {
            if answered.has_valid() {
                break;
            }
            stage = Some(candidate.stage);
        }
        answered.push(candidate.classify(query, from));
    }

    answered
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
/// Inner  find_member(Inner)  hit   -> type, Parsed arm
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
            find_member(file, *declaration, name, model::Namespace::Type)
                .into_iter()
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
        return ClassifiedTypeCandidate::Invalid(InvalidTypeCandidate {
            target,
            reasons: vec![TypeInvalidity::OutsideScope],
        });
    }

    if type_target_is_accessible(&target, query, from) {
        ClassifiedTypeCandidate::Valid(target)
    } else {
        ClassifiedTypeCandidate::Invalid(InvalidTypeCandidate {
            target,
            reasons: vec![TypeInvalidity::Inaccessible],
        })
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
            is_compiled_type_accessible(query.class_access(source, fqn), fqn.package(), from)
        }
    }
}

fn classify_candidates(
    candidates: impl IntoIterator<Item = TypeTarget>,
    invalid_candidates: Vec<InvalidTypeCandidate>,
) -> TypeResolution {
    let mut distinct = Vec::new();
    for candidate in candidates {
        if !distinct.contains(&candidate) {
            distinct.push(candidate);
        }
    }

    match distinct.len() {
        0 => TypeResolution::Unresolved { invalid_candidates },
        1 => TypeResolution::Resolved(distinct.pop().unwrap()),
        _ => TypeResolution::Ambiguous(distinct),
    }
}

/// The members of a type with a matching name in the given namespace.
/// No inheritance yet: only the type's own body scope is searched.
fn find_member(
    file: &model::File,
    type_declaration: model::DeclarationId,
    name: &model::Identifier,
    namespace: model::Namespace,
) -> Vec<model::DeclarationId> {
    let model::Declaration::Type(declaration) = &file.declarations[type_declaration.0] else {
        return Vec::new();
    };

    file.lexical_scopes[declaration.body_scope.0]
        .declarations
        .iter()
        .copied()
        .filter(|member_id| {
            let member = &file.declarations[member_id.0];
            member.namespace() == namespace && member.name().is_some_and(|n| n.text == name.text)
        })
        .collect()
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

    let targets = match entity {
        model::EntityId::Declaration(declaration) => vec![(source.clone(), declaration)],
        model::EntityId::TypeRef(owner) => {
            let declaration = &file.declarations[owner.0];
            let Some(type_ref) = declaration.type_ref() else {
                return Vec::new();
            };
            resolve_type_reference(source, file, type_ref, declaration.declaring_scope(), query)
        }
        model::EntityId::BodyNode(body, node) => {
            let model::BodyNodeKind::Expression(_) = &file.bodies[body.0].node(node).kind else {
                return Vec::new();
            };
            resolve_expression(source, file, body, node, query)
        }
        model::EntityId::Scope(..) | model::EntityId::Import(..) => Vec::new(),
    };

    targets
        .iter()
        .filter_map(|(target_source, declaration_id)| {
            let target_file = query.model_of(target_source)?;
            let span = target_file.declarations[declaration_id.0].name_span()?;
            Some(NavigationTarget {
                source: target_source.clone(),
                span,
            })
        })
        .collect()
}

pub(crate) fn resolve_expression(
    source: &jvm::model::Source,
    file: &model::File,
    body_id: model::BodyId,
    expression_id: model::BodyNodeId,
    query: &Query,
) -> Vec<(jvm::model::Source, model::DeclarationId)> {
    let body = &file.bodies[body_id.0];
    let scope = body.node(expression_id).scope;
    let Some(expression) = body.expression(expression_id) else {
        return Vec::new();
    };
    match expression {
        model::Expression::NameRef { name } => resolve_variable_name(file, name, scope)
            .into_iter()
            .map(|declaration| (source.clone(), declaration))
            .collect(),
        model::Expression::This => file
            .enclosing_type_declaration(scope)
            .map(|declaration| vec![(source.clone(), declaration)])
            .unwrap_or_default(),
        model::Expression::FieldAccess { receiver, name } => {
            let Some((class_source, class)) =
                resolve_receiver_class(source, file, body_id, *receiver, query)
            else {
                return Vec::new();
            };
            let Some(class_file) = query.model_of(&class_source) else {
                return Vec::new();
            };
            find_member(class_file, class, name, model::Namespace::Variable)
                .into_iter()
                .map(|member| (class_source.clone(), member))
                .collect()
        }
        model::Expression::MethodCall { receiver, name, .. } => {
            let receiver_class = match receiver {
                Some(receiver) => resolve_receiver_class(source, file, body_id, *receiver, query),
                None => file
                    .enclosing_type_declaration(scope)
                    .map(|declaration| (source.clone(), declaration)),
            };
            let Some((class_source, class)) = receiver_class else {
                return Vec::new();
            };
            let Some(class_file) = query.model_of(&class_source) else {
                return Vec::new();
            };
            find_member(class_file, class, name, model::Namespace::Method)
                .into_iter()
                .map(|member| (class_source.clone(), member))
                .collect()
        }
        model::Expression::ObjectCreation { ty, .. } => {
            resolve_type_reference(source, file, ty, scope, query)
        }
        model::Expression::Assign { .. } | model::Expression::Literal => Vec::new(),
    }
}

/// The class through which member lookup for `expression` runs: the declared
/// type of the expression, or the type itself for static access (`Bar.asd`).
fn resolve_receiver_class(
    source: &jvm::model::Source,
    file: &model::File,
    body_id: model::BodyId,
    expression_id: model::BodyNodeId,
    query: &Query,
) -> Option<(jvm::model::Source, model::DeclarationId)> {
    let body = &file.bodies[body_id.0];
    let scope = body.node(expression_id).scope;
    let Some(expression) = body.expression(expression_id) else {
        return None;
    };
    match expression {
        model::Expression::This => {
            let declaration = file.enclosing_type_declaration(scope)?;
            Some((source.clone(), declaration))
        }
        model::Expression::NameRef { name } => {
            if let Some(variable) = resolve_variable_name(file, name, scope).into_iter().next() {
                let declaration = &file.declarations[variable.0];
                let type_ref = declaration.type_ref()?;
                return resolve_type_reference(
                    source,
                    file,
                    type_ref,
                    declaration.declaring_scope(),
                    query,
                )
                .into_iter()
                .next();
            }

            // Not a variable: try a type name for static access (`Bar.asd`).
            match resolve_type_name(
                &model::Name::Simple(name.clone()),
                source,
                file,
                scope,
                query,
            ) {
                TypeResolution::Resolved(TypeTarget::Parsed {
                    source,
                    declaration,
                }) => Some((source, declaration)),
                _ => None,
            }
        }
        model::Expression::FieldAccess { receiver, name } => {
            let (class_source, class) =
                resolve_receiver_class(source, file, body_id, *receiver, query)?;
            let class_file = query.model_of(&class_source)?;
            let member = find_member(class_file, class, name, model::Namespace::Variable)
                .into_iter()
                .next()?;
            let declaration = &class_file.declarations[member.0];
            resolve_type_reference(
                &class_source,
                class_file,
                declaration.type_ref()?,
                declaration.declaring_scope(),
                query,
            )
            .into_iter()
            .next()
        }
        model::Expression::MethodCall { receiver, name, .. } => {
            let receiver_class = match receiver {
                Some(receiver) => resolve_receiver_class(source, file, body_id, *receiver, query),
                None => file
                    .enclosing_type_declaration(scope)
                    .map(|declaration| (source.clone(), declaration)),
            }?;
            let (class_source, class) = receiver_class;
            let class_file = query.model_of(&class_source)?;
            let member = find_member(class_file, class, name, model::Namespace::Method)
                .into_iter()
                .next()?;
            let declaration = &class_file.declarations[member.0];
            resolve_type_reference(
                &class_source,
                class_file,
                declaration.type_ref()?,
                declaration.declaring_scope(),
                query,
            )
            .into_iter()
            .next()
        }
        model::Expression::ObjectCreation { ty, .. } => {
            resolve_type_reference(source, file, ty, scope, query)
                .into_iter()
                .next()
        }
        model::Expression::Assign { .. } | model::Expression::Literal => None,
    }
}

/// A syntactic type annotation resolved to its declaring class.
fn resolve_type_reference(
    source: &jvm::model::Source,
    file: &model::File,
    type_ref: &model::TypeRef,
    scope: model::LexicalScopeId,
    query: &Query,
) -> Vec<(jvm::model::Source, model::DeclarationId)> {
    if type_ref.primitive {
        return Vec::new();
    }

    match resolve_type_name(&type_ref.name, source, file, scope, query) {
        TypeResolution::Resolved(TypeTarget::Parsed {
            source,
            declaration,
        }) => vec![(source, declaration)],
        TypeResolution::Ambiguous(targets) => targets
            .into_iter()
            .filter_map(|target| match target {
                TypeTarget::Parsed {
                    source,
                    declaration,
                } => Some((source, declaration)),
                TypeTarget::Compiled { .. } => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// A bare name in expression position: locals, parameters, then fields,
/// nearest scope first. Always in-file.
pub(crate) fn resolve_variable_name(
    file: &model::File,
    name: &model::Identifier,
    scope: model::LexicalScopeId,
) -> Vec<model::DeclarationId> {
    for (_, scope) in file.iter_scope_chain(scope) {
        let hits: Vec<model::DeclarationId> = scope
            .declarations
            .iter()
            .copied()
            .filter(|declaration_id| {
                match &file.declarations[declaration_id.0] {
                    // JLS 6.3: a local's scope starts at its declarator.
                    model::Declaration::Local(declaration) => {
                        declaration.name.as_ref().is_some_and(|local| {
                            local.text == name.text && local.span.start <= name.span.start
                        })
                    }
                    model::Declaration::Parameter(_) | model::Declaration::Field(_) => file
                        .declarations[declaration_id.0]
                        .name()
                        .is_some_and(|candidate| candidate.text == name.text),
                    _ => false,
                }
            })
            .collect();
        if !hits.is_empty() {
            return hits;
        }
    }

    Vec::new()
}

#[cfg(test)]
mod tests;
