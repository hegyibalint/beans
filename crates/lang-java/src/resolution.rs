use beans_core::language::NavigationTarget;
use beans_core::model::Offset;
use beans_platform_jvm::{
    model::{JvmQualifiedName, JvmSource},
    query::JvmScopeMembership,
};

use crate::{
    accessibility::{JavaSite, is_accessible},
    model::{
        JavaBodyId, JavaBodyNodeId, JavaBodyNodeKind, JavaDeclaration, JavaDeclarationId,
        JavaEntityId, JavaExpression, JavaFile, JavaIdentifier, JavaImport, JavaImportKind,
        JavaLexicalScopeId, JavaName, JavaNamespace, JavaTypeRef,
    },
    query::JavaQuery,
};

/// The main result of resolving a type name.
///
/// The important thing is that we separate and supply candidates where we can.
/// This allows us to make better decisions and actions when we have multiple candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaTypeResolution {
    /// We couldn't find anything that resembles the type in need.
    /// As a consequence, we cannot supply any supplimental information
    Unresolved {
        invalid_candidates: Vec<InvalidJavaTypeCandidate>,
    },
    /// We had a successful resolution, with a single candidate.
    /// Our happy path.
    Resolved(JavaTypeTarget),
    /// We had a _too_ successful resolution, and we found more than one candidate.
    /// We store all candidates, so we can use this information further down the line.
    Ambiguous(Vec<JavaTypeTarget>),
}

impl JavaTypeResolution {
    pub(crate) fn has_invalidity(&self, reason: JavaTypeInvalidity) -> bool {
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
pub enum JavaTypeTarget {
    /// The resolution target is coming from the `lang-java` land.
    /// We store the raw declaration ID, as we have access to the model and its declarations.
    Java {
        source: JvmSource,
        declaration: JavaDeclarationId,
    },
    /// The resulution target is coming from the `platform-jvm` land (i.e. anything but a Java source file).
    /// We store the "coordinate" pointing at the resource (e.g., a `.class`, a `.jar`, etc) and the FQN in there what we resolved to.
    Jvm {
        source: JvmSource,
        fqn: JvmQualifiedName,
    },
}

impl JavaTypeTarget {
    pub(crate) fn source(&self) -> &JvmSource {
        match self {
            Self::Java { source, .. } | Self::Jvm { source, .. } => source,
        }
    }
}

/// Resolution might find candidates, however, that doesn't mean that those candidates are valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum JavaTypeInvalidity {
    /// The candidate we found is not in the scope of this module.
    /// Think of this as a `main` module using a class from `test`; normally this direction is set up to be impossible.
    OutsideScope,
    /// The candidate we found is in scope, but its access modifiers prohibit us from using it.
    /// Think of using a private class from another package; resolution sees the class, but it understands that
    Inaccessible,
}

/// Represents a resolution candidate that was rejected for some reason
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InvalidJavaTypeCandidate {
    target: JavaTypeTarget,
    reasons: Vec<JavaTypeInvalidity>,
}

impl InvalidJavaTypeCandidate {
    pub(crate) fn target(&self) -> &JavaTypeTarget {
        &self.target
    }

    pub(crate) fn invalidities(&self) -> &[JavaTypeInvalidity] {
        &self.reasons
    }

    pub(crate) fn has_invalidity(&self, reason: JavaTypeInvalidity) -> bool {
        self.invalidities().contains(&reason)
    }

    fn add_reason(&mut self, reason: JavaTypeInvalidity) {
        if !self.has_invalidity(reason) {
            self.reasons.push(reason);
            self.reasons.sort_unstable();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ClassifiedJavaTypeCandidate {
    Valid(JavaTypeTarget),
    Invalid(InvalidJavaTypeCandidate),
}

impl ClassifiedJavaTypeCandidate {
    fn target(&self) -> &JavaTypeTarget {
        match self {
            Self::Valid(target) => target,
            Self::Invalid(candidate) => candidate.target(),
        }
    }

    fn propagate(self, child: ClassifiedJavaTypeCandidate) -> ClassifiedJavaTypeCandidate {
        let Self::Invalid(parent) = self else {
            return child;
        };

        let mut child = match child {
            Self::Valid(target) => InvalidJavaTypeCandidate {
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
    valid: Vec<JavaTypeTarget>,
    invalid: Vec<InvalidJavaTypeCandidate>,
}

impl ResolutionCandidates {
    fn from_valid(candidates: impl IntoIterator<Item = JavaTypeTarget>) -> Self {
        candidates
            .into_iter()
            .map(ClassifiedJavaTypeCandidate::Valid)
            .collect()
    }

    fn push(&mut self, candidate: ClassifiedJavaTypeCandidate) {
        match candidate {
            ClassifiedJavaTypeCandidate::Valid(target) => {
                if !self.valid.contains(&target) {
                    self.valid.push(target);
                }
            }
            ClassifiedJavaTypeCandidate::Invalid(candidate) => {
                if !self.invalid.contains(&candidate) {
                    self.invalid.push(candidate);
                }
            }
        }
    }

    fn or_stage(mut self, next: impl FnOnce() -> Self) -> Self {
        if !self.valid.is_empty() {
            return self;
        }

        let next = next();
        self.valid = next.valid;
        for candidate in next.invalid {
            self.push(ClassifiedJavaTypeCandidate::Invalid(candidate));
        }
        self
    }

    fn into_resolution(self) -> JavaTypeResolution {
        classify_candidates(self.valid, self.invalid)
    }

    fn commits_type_path(&self) -> bool {
        self.has_valid()
            || self
                .invalid
                .iter()
                .any(|candidate| candidate.has_invalidity(JavaTypeInvalidity::Inaccessible))
    }

    pub(crate) fn has_valid(&self) -> bool {
        !self.valid.is_empty()
    }

    #[cfg(test)]
    fn has_invalidity(&self, reason: JavaTypeInvalidity) -> bool {
        self.invalid
            .iter()
            .any(|candidate| candidate.has_invalidity(reason))
    }
}

impl FromIterator<ClassifiedJavaTypeCandidate> for ResolutionCandidates {
    fn from_iter<T: IntoIterator<Item = ClassifiedJavaTypeCandidate>>(iter: T) -> Self {
        let mut candidates = Self::default();
        for candidate in iter {
            candidates.push(candidate);
        }
        candidates
    }
}

impl IntoIterator for ResolutionCandidates {
    type Item = ClassifiedJavaTypeCandidate;
    type IntoIter = std::vec::IntoIter<ClassifiedJavaTypeCandidate>;

    fn into_iter(self) -> Self::IntoIter {
        self.valid
            .into_iter()
            .map(ClassifiedJavaTypeCandidate::Valid)
            .chain(
                self.invalid
                    .into_iter()
                    .map(ClassifiedJavaTypeCandidate::Invalid),
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
    name: &JavaName,
    source: &JvmSource,
    file: &JavaFile,
    current_lexical_scope_id: JavaLexicalScopeId,
    query: &JavaQuery,
) -> JavaTypeResolution {
    resolve_type_candidates(name, source, file, current_lexical_scope_id, query).into_resolution()
}

pub(crate) fn resolve_type_candidates(
    name: &JavaName,
    source: &JvmSource,
    file: &JavaFile,
    current_lexical_scope_id: JavaLexicalScopeId,
    query: &JavaQuery,
) -> ResolutionCandidates {
    // §6.5.5.2. A qualified name classifies its prefix as a package or a type
    // first; `resolve_canonical_name` walks that, but only for import names so
    // far.
    let JavaName::Simple(name) = name else {
        return ResolutionCandidates::default();
    };

    let from = JavaSite {
        source,
        file,
        scope: current_lexical_scope_id,
    };

    // Stage 1. Type parameters, member types and local types, nearest scope
    // first. §6.4.1: a type declaration shadows every other type of that name
    // in scope where it occurs, which is what makes nearest-first right.
    candidates_from_lexical_scopes(name, source, file, current_lexical_scope_id)
        // Stage 2. §7.5.1 rejects an inaccessible import. For useful recovery,
        // follow javac: retain that failure without letting it hide an
        // accessible answer from a lower stage.
        .or_stage(|| candidates_from_exact_imports(name, source, file, query))
        // Stage 3. Top-level types of the current package, in scope by §6.3.
        .or_stage(|| candidates_from_same_package(name, &from, query))
    // Stages 4–6 (on-demand, module, suggestions) are not implemented. Their
    // absence means no valid candidate, never a guess.
}

fn candidates_from_lexical_scopes(
    name: &JavaIdentifier,
    source: &JvmSource,
    file: &JavaFile,
    current_lexical_scope_id: JavaLexicalScopeId,
) -> ResolutionCandidates {
    for (_, scope) in file.iter_scope_chain(current_lexical_scope_id) {
        let candidates = scope
            .declarations
            .iter()
            .copied()
            .filter_map(|declaration_id| {
                let declaration = file.declarations.get(declaration_id.0)?;
                let declaration_name = match declaration {
                    JavaDeclaration::Type(declaration) => declaration.name.as_ref(),
                    JavaDeclaration::TypeParameter(declaration) => Some(&declaration.name),
                    _ => None,
                }?;

                (declaration_name.text == name.text).then(|| JavaTypeTarget::Java {
                    source: source.clone(),
                    declaration: declaration_id,
                })
            });
        let candidates = ResolutionCandidates::from_valid(candidates);
        if candidates.has_valid() {
            return candidates;
        }
    }

    ResolutionCandidates::default()
}

#[cfg(test)]
fn resolve_type_from_lexical_scopes(
    name: &JavaIdentifier,
    source: &JvmSource,
    file: &JavaFile,
    current_lexical_scope_id: JavaLexicalScopeId,
) -> JavaTypeResolution {
    candidates_from_lexical_scopes(name, source, file, current_lexical_scope_id).into_resolution()
}

fn candidates_from_exact_imports(
    name: &JavaIdentifier,
    source: &JvmSource,
    file: &JavaFile,
    query: &JavaQuery,
) -> ResolutionCandidates {
    let from = JavaSite {
        source,
        file,
        scope: JavaLexicalScopeId(0),
    };

    file.imports
        .iter()
        .filter(|import| import.kind == JavaImportKind::Type)
        .filter(|import| exact_import_introduces_name(import, name))
        .flat_map(|import| resolve_canonical_name(&import.name, &from, query))
        .collect()
}

#[cfg(test)]
fn resolve_type_from_exact_imports(
    name: &JavaIdentifier,
    source: &JvmSource,
    file: &JavaFile,
    query: &JavaQuery,
) -> JavaTypeResolution {
    candidates_from_exact_imports(name, source, file, query).into_resolution()
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
/// Outer  probe p.Outer       hit   -> type, Java arm
/// Inner  find_member(Inner)  hit   -> type, Java arm
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
/// Map    probe java.util.Map        hit   -> type, Jvm arm
/// Entry  probe java.util.Map$Entry  hit   -> type, Jvm arm
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
fn resolve_canonical_name(
    name: &JavaName,
    from: &JavaSite,
    query: &JavaQuery,
) -> ResolutionCandidates {
    let mut paths = vec![CanonicalPath::Package(String::new())];

    for segment in name.segments() {
        let mut next = Vec::new();
        for path in paths {
            match path {
                CanonicalPath::Package(prefix) => {
                    let name = JvmQualifiedName::in_package(&prefix, &segment.text);
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
    Type(ClassifiedJavaTypeCandidate),
}

/// §6.5.4.2's TypeName branch: member types before scope and access are
/// composed. A file this vertical parsed holds its members in the model;
/// anything else holds them in the lake under a nested binary name.
fn member_types(
    target: &JavaTypeTarget,
    name: &JavaIdentifier,
    query: &JavaQuery,
) -> Vec<JavaTypeTarget> {
    match target {
        JavaTypeTarget::Java {
            source,
            declaration,
        } => {
            let Some(file) = query.model_of(source) else {
                return Vec::new();
            };
            find_member(file, *declaration, name, JavaNamespace::Type)
                .into_iter()
                .map(|declaration| JavaTypeTarget::Java {
                    source: source.clone(),
                    declaration,
                })
                .collect()
        }
        JavaTypeTarget::Jvm { fqn, .. } => query.types_named(&fqn.nested(&name.text)),
    }
}

fn exact_import_introduces_name(import: &JavaImport, name: &JavaIdentifier) -> bool {
    import
        .name
        .segments()
        .last()
        .is_some_and(|imported| imported.text == name.text)
}

/// The current package plus the simple name is the binary name the lake was
/// told about at projection time, so this is one lookup rather than a walk
/// over every file comparing package declarations.
fn candidates_from_same_package(
    name: &JavaIdentifier,
    from: &JavaSite,
    query: &JavaQuery,
) -> ResolutionCandidates {
    let package = from
        .file
        .package
        .as_ref()
        .map(JavaName::dotted)
        .unwrap_or_default();

    classify_types_named(
        query,
        &JvmQualifiedName::in_package(&package, &name.text),
        from,
    )
}

#[cfg(test)]
fn resolve_from_same_package(
    name: &JavaIdentifier,
    source: &JvmSource,
    file: &JavaFile,
    query: &JavaQuery,
) -> JavaTypeResolution {
    candidates_from_same_package(
        name,
        &JavaSite {
            source,
            file,
            scope: JavaLexicalScopeId(0),
        },
        query,
    )
    .into_resolution()
}

fn classify_types_named(
    query: &JavaQuery,
    fqn: &JvmQualifiedName,
    from: &JavaSite,
) -> ResolutionCandidates {
    query
        .types_named(fqn)
        .into_iter()
        .map(|target| classify_type_target(target, query, from))
        .collect()
}

fn classify_type_target(
    target: JavaTypeTarget,
    query: &JavaQuery,
    from: &JavaSite,
) -> ClassifiedJavaTypeCandidate {
    if query.scope_membership(&target) == JvmScopeMembership::OutsideScope {
        return ClassifiedJavaTypeCandidate::Invalid(InvalidJavaTypeCandidate {
            target,
            reasons: vec![JavaTypeInvalidity::OutsideScope],
        });
    }

    if type_target_is_accessible(&target, query, from) {
        ClassifiedJavaTypeCandidate::Valid(target)
    } else {
        ClassifiedJavaTypeCandidate::Invalid(InvalidJavaTypeCandidate {
            target,
            reasons: vec![JavaTypeInvalidity::Inaccessible],
        })
    }
}

fn type_target_is_accessible(target: &JavaTypeTarget, query: &JavaQuery, from: &JavaSite) -> bool {
    let JavaTypeTarget::Java {
        source,
        declaration,
    } = target
    else {
        return true;
    };
    let Some(file) = query.model_of(source) else {
        return true;
    };
    let JavaDeclaration::Type(declaration) = &file.declarations[declaration.0] else {
        return true;
    };
    let declared = JavaSite {
        source,
        file,
        scope: declaration.declaring_scope,
    };

    is_accessible(declaration.access, &declared, from)
}

fn classify_candidates(
    candidates: impl IntoIterator<Item = JavaTypeTarget>,
    invalid_candidates: Vec<InvalidJavaTypeCandidate>,
) -> JavaTypeResolution {
    let mut distinct = Vec::new();
    for candidate in candidates {
        if !distinct.contains(&candidate) {
            distinct.push(candidate);
        }
    }

    match distinct.len() {
        0 => JavaTypeResolution::Unresolved { invalid_candidates },
        1 => JavaTypeResolution::Resolved(distinct.pop().unwrap()),
        _ => JavaTypeResolution::Ambiguous(distinct),
    }
}

/// The members of a type with a matching name in the given namespace.
/// No inheritance yet: only the type's own body scope is searched.
fn find_member(
    file: &JavaFile,
    type_declaration: JavaDeclarationId,
    name: &JavaIdentifier,
    namespace: JavaNamespace,
) -> Vec<JavaDeclarationId> {
    let JavaDeclaration::Type(declaration) = &file.declarations[type_declaration.0] else {
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
    source: &JvmSource,
    file: &JavaFile,
    offset: Offset,
    query: &JavaQuery,
) -> Vec<NavigationTarget<JvmSource>> {
    let Some((_, entity)) = file.position_index.tightest_containing(offset) else {
        return Vec::new();
    };

    let targets = match entity {
        JavaEntityId::Declaration(declaration) => vec![(source.clone(), declaration)],
        JavaEntityId::TypeRef(owner) => {
            let declaration = &file.declarations[owner.0];
            let Some(type_ref) = declaration.type_ref() else {
                return Vec::new();
            };
            resolve_type_reference(source, file, type_ref, declaration.declaring_scope(), query)
        }
        JavaEntityId::BodyNode(body, node) => {
            let JavaBodyNodeKind::Expression(_) = &file.bodies[body.0].node(node).kind else {
                return Vec::new();
            };
            resolve_expression(source, file, body, node, query)
        }
        JavaEntityId::Scope(..) | JavaEntityId::Import(..) => Vec::new(),
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
    source: &JvmSource,
    file: &JavaFile,
    body_id: JavaBodyId,
    expression_id: JavaBodyNodeId,
    query: &JavaQuery,
) -> Vec<(JvmSource, JavaDeclarationId)> {
    let body = &file.bodies[body_id.0];
    let scope = body.node(expression_id).scope;
    let Some(expression) = body.expression(expression_id) else {
        return Vec::new();
    };
    match expression {
        JavaExpression::NameRef { name } => resolve_variable_name(file, name, scope)
            .into_iter()
            .map(|declaration| (source.clone(), declaration))
            .collect(),
        JavaExpression::This => file
            .enclosing_type_declaration(scope)
            .map(|declaration| vec![(source.clone(), declaration)])
            .unwrap_or_default(),
        JavaExpression::FieldAccess { receiver, name } => {
            let Some((class_source, class)) =
                resolve_receiver_class(source, file, body_id, *receiver, query)
            else {
                return Vec::new();
            };
            let Some(class_file) = query.model_of(&class_source) else {
                return Vec::new();
            };
            find_member(class_file, class, name, JavaNamespace::Variable)
                .into_iter()
                .map(|member| (class_source.clone(), member))
                .collect()
        }
        JavaExpression::MethodCall { receiver, name, .. } => {
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
            find_member(class_file, class, name, JavaNamespace::Method)
                .into_iter()
                .map(|member| (class_source.clone(), member))
                .collect()
        }
        JavaExpression::ObjectCreation { ty, .. } => {
            resolve_type_reference(source, file, ty, scope, query)
        }
        JavaExpression::Assign { .. } | JavaExpression::Literal => Vec::new(),
    }
}

/// The class through which member lookup for `expression` runs: the declared
/// type of the expression, or the type itself for static access (`Bar.asd`).
fn resolve_receiver_class(
    source: &JvmSource,
    file: &JavaFile,
    body_id: JavaBodyId,
    expression_id: JavaBodyNodeId,
    query: &JavaQuery,
) -> Option<(JvmSource, JavaDeclarationId)> {
    let body = &file.bodies[body_id.0];
    let scope = body.node(expression_id).scope;
    let Some(expression) = body.expression(expression_id) else {
        return None;
    };
    match expression {
        JavaExpression::This => {
            let declaration = file.enclosing_type_declaration(scope)?;
            Some((source.clone(), declaration))
        }
        JavaExpression::NameRef { name } => {
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
            match resolve_type_name(&JavaName::Simple(name.clone()), source, file, scope, query) {
                JavaTypeResolution::Resolved(JavaTypeTarget::Java {
                    source,
                    declaration,
                }) => Some((source, declaration)),
                _ => None,
            }
        }
        JavaExpression::FieldAccess { receiver, name } => {
            let (class_source, class) =
                resolve_receiver_class(source, file, body_id, *receiver, query)?;
            let class_file = query.model_of(&class_source)?;
            let member = find_member(class_file, class, name, JavaNamespace::Variable)
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
        JavaExpression::MethodCall { receiver, name, .. } => {
            let receiver_class = match receiver {
                Some(receiver) => resolve_receiver_class(source, file, body_id, *receiver, query),
                None => file
                    .enclosing_type_declaration(scope)
                    .map(|declaration| (source.clone(), declaration)),
            }?;
            let (class_source, class) = receiver_class;
            let class_file = query.model_of(&class_source)?;
            let member = find_member(class_file, class, name, JavaNamespace::Method)
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
        JavaExpression::ObjectCreation { ty, .. } => {
            resolve_type_reference(source, file, ty, scope, query)
                .into_iter()
                .next()
        }
        JavaExpression::Assign { .. } | JavaExpression::Literal => None,
    }
}

/// A syntactic type annotation resolved to its declaring class.
fn resolve_type_reference(
    source: &JvmSource,
    file: &JavaFile,
    type_ref: &JavaTypeRef,
    scope: JavaLexicalScopeId,
    query: &JavaQuery,
) -> Vec<(JvmSource, JavaDeclarationId)> {
    if type_ref.primitive {
        return Vec::new();
    }

    match resolve_type_name(&type_ref.name, source, file, scope, query) {
        JavaTypeResolution::Resolved(JavaTypeTarget::Java {
            source,
            declaration,
        }) => vec![(source, declaration)],
        JavaTypeResolution::Ambiguous(targets) => targets
            .into_iter()
            .filter_map(|target| match target {
                JavaTypeTarget::Java {
                    source,
                    declaration,
                } => Some((source, declaration)),
                JavaTypeTarget::Jvm { .. } => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// A bare name in expression position: locals, parameters, then fields,
/// nearest scope first. Always in-file.
pub(crate) fn resolve_variable_name(
    file: &JavaFile,
    name: &JavaIdentifier,
    scope: JavaLexicalScopeId,
) -> Vec<JavaDeclarationId> {
    for (_, scope) in file.iter_scope_chain(scope) {
        let hits: Vec<JavaDeclarationId> = scope
            .declarations
            .iter()
            .copied()
            .filter(|declaration_id| {
                match &file.declarations[declaration_id.0] {
                    // JLS 6.3: a local's scope starts at its declarator.
                    JavaDeclaration::Local(declaration) => {
                        declaration.name.as_ref().is_some_and(|local| {
                            local.text == name.text && local.span.start <= name.span.start
                        })
                    }
                    JavaDeclaration::Parameter(_) | JavaDeclaration::Field(_) => file.declarations
                        [declaration_id.0]
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
