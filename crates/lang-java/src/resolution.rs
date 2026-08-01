use beans_core::language::NavigationTarget;
use beans_core::model::Offset;
use beans_platform_jvm::{
    model::{JvmQualifiedName, JvmSource},
    query::JvmScopeMembership,
};

use crate::{
    model::{
        JavaBodyId, JavaBodyNodeId, JavaBodyNodeKind, JavaDeclaration, JavaDeclarationId,
        JavaEntityId, JavaExpression, JavaFile, JavaIdentifier, JavaImport, JavaImportKind,
        JavaLexicalScopeId, JavaName, JavaNamespace, JavaTypeRef,
    },
    query::JavaQuery,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaTypeTarget {
    Java {
        source: JvmSource,
        declaration: JavaDeclarationId,
    },
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaTypeResolution {
    Resolved(JavaTypeTarget),
    Ambiguous(Vec<JavaTypeTarget>),
    Unresolved,
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
    // §6.5.5.2. A qualified name classifies its prefix as a package or a type
    // first; `resolve_canonical_name` walks that, but only for import names so
    // far.
    let JavaName::Simple(name) = name else {
        return JavaTypeResolution::Unresolved;
    };

    // Stage 1. Type parameters, member types and local types, nearest scope
    // first. §6.4.1: a type declaration shadows every other type of that name
    // in scope where it occurs, which is what makes nearest-first right.
    let lexical = resolve_type_from_lexical_scopes(name, source, file, current_lexical_scope_id);
    if !matches!(lexical, JavaTypeResolution::Unresolved) {
        return lexical;
    }

    // Stage 2. Single-type (§7.5.1) and single-static (§7.5.3) imports. §6.4.1
    // has a single-type import shadow a top-level type of that name in
    // *another* compilation unit of this package, so it outranks stage 3 while
    // stage 1 keeps whatever this file declares.
    let exact_import = resolve_type_from_exact_imports(name, file, query);
    if !matches!(exact_import, JavaTypeResolution::Unresolved) {
        return exact_import;
    }

    // Stage 3. Top-level types of the current package, in scope by §6.3: "all
    // the class and interface declarations in the package in which the top
    // level class or interface is declared".
    let same_package = resolve_from_same_package(name, file, query);
    if !matches!(same_package, JavaTypeResolution::Unresolved) {
        return same_package;
    }

    // Stage 4. Type-import-on-demand (§7.5.2), static-import-on-demand
    // (§7.5.4), and the implicit `import java.lang.*` (§7.3). Below stage 3 on
    // §7.5.2's own reading, where an on-demand import is shadowed "by a class
    // or interface [...] declared in the package to which the compilation unit
    // belongs". That sentence is example prose, not a rule in §6.4.1, which
    // says nothing about this pair.
    // Stage 5. Single-module imports (§7.5.5). Last because §6.4.1 has both
    // on-demand forms shadow them and has them shadow nothing at all.
    // Stage 6. Import suggestions. No JLS rule: the spec ends a failed lookup
    // in a compile-time error, and offering to fix it is ours.
    // Not implemented yet: absence of the later stages means unresolved,
    // never a guess.
    JavaTypeResolution::Unresolved
}

fn resolve_type_from_lexical_scopes(
    name: &JavaIdentifier,
    source: &JvmSource,
    file: &JavaFile,
    current_lexical_scope_id: JavaLexicalScopeId,
) -> JavaTypeResolution {
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
        let resolution = classify_candidates(candidates);
        if !matches!(resolution, JavaTypeResolution::Unresolved) {
            return resolution;
        }
    }

    JavaTypeResolution::Unresolved
}

fn resolve_type_from_exact_imports(
    name: &JavaIdentifier,
    file: &JavaFile,
    query: &JavaQuery,
) -> JavaTypeResolution {
    let candidates = file
        .imports
        .iter()
        .filter(|import| import.kind == JavaImportKind::Type)
        .filter(|import| exact_import_introduces_name(import, name))
        .flat_map(|import| resolve_canonical_name(&import.name, query));

    classify_candidates(candidates)
}

/// A dotted name hides where the package stops and the type begins. JLS 26
/// §6.5.4.2 answers it with one question asked left to right: does whatever `Q`
/// denotes so far have a member class or interface named `Id`? A package and a
/// type are the same question there, so the walk below only has to remember
/// which of the two it is holding. It starts on the unnamed package (§6.5.4.1)
/// and switches to type mode the first time the answer is yes. Segments join
/// with `.` before the switch and with `$` after it (§13.1). Nothing is guessed
/// and no prefix is tried twice.
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
fn resolve_canonical_name(name: &JavaName, query: &JavaQuery) -> Vec<JavaTypeTarget> {
    let mut mode = Mode::Package(String::new());

    for segment in name.segments() {
        mode = match mode {
            Mode::Package(prefix) => {
                let candidate = JvmQualifiedName::in_package(&prefix, &segment.text);
                let targets = types_named_in_scope(query, &candidate);
                if targets.is_empty() {
                    Mode::Package(candidate.as_str().to_owned())
                } else {
                    Mode::Type(targets)
                }
            }
            Mode::Type(targets) => Mode::Type(
                targets
                    .into_iter()
                    .flat_map(|target| member_types(target, segment, query))
                    .collect(),
            ),
        };
    }

    match mode {
        Mode::Type(targets) => targets,
        Mode::Package(_) => Vec::new(),
    }
}

enum Mode {
    /// The dotted package name walked so far; empty is the unnamed package.
    Package(String),
    /// What the switch to type mode produced. There is no way back, so an
    /// empty set here means the rest of the name has nothing to attach to.
    Type(Vec<JavaTypeTarget>),
}

/// §6.5.4.2's TypeName branch: a member type, or nothing. A file this vertical
/// parsed holds its members in the model; anything else holds them in the lake
/// under a nested binary name.
fn member_types(
    target: JavaTypeTarget,
    name: &JavaIdentifier,
    query: &JavaQuery,
) -> Vec<JavaTypeTarget> {
    match target {
        JavaTypeTarget::Java {
            source,
            declaration,
        } => {
            let Some(file) = query.model_of(&source) else {
                return Vec::new();
            };
            find_member(file, declaration, name, JavaNamespace::Type)
                .into_iter()
                .map(|declaration| JavaTypeTarget::Java {
                    source: source.clone(),
                    declaration,
                })
                .collect()
        }
        JavaTypeTarget::Jvm { fqn, .. } => types_named_in_scope(query, &fqn.nested(&name.text)),
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
fn resolve_from_same_package(
    name: &JavaIdentifier,
    file: &JavaFile,
    query: &JavaQuery,
) -> JavaTypeResolution {
    let package = file
        .package
        .as_ref()
        .map(JavaName::dotted)
        .unwrap_or_default();

    classify_candidates(types_named_in_scope(
        query,
        &JvmQualifiedName::in_package(&package, &name.text),
    ))
}

fn types_named_in_scope(query: &JavaQuery, fqn: &JvmQualifiedName) -> Vec<JavaTypeTarget> {
    query
        .types_named(fqn)
        .into_iter()
        .filter(|target| query.scope_membership(target) == JvmScopeMembership::InScope)
        .collect()
}

fn classify_candidates(candidates: impl IntoIterator<Item = JavaTypeTarget>) -> JavaTypeResolution {
    let mut distinct = Vec::new();
    for candidate in candidates {
        if !distinct.contains(&candidate) {
            distinct.push(candidate);
        }
    }

    match distinct.len() {
        0 => JavaTypeResolution::Unresolved,
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
