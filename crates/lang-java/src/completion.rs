use beans_core::language::{CompletionItem, CompletionItemKind, Handle};
use beans_core::model::{Offset, OffsetSpan};
use beans_core::storage::Revision;
use beans_platform_jvm as jvm;

use crate::accessibility::Site;
use crate::model;
use crate::query::Query;
use crate::resolution::{
    ResolutionCandidates, TypeTarget, candidates_from_java_lang, candidates_from_same_package,
};

/// JLS 26 §6.3 names what completion is asked about:
///
/// > A declaration is said to be *in scope* at a particular point in a program
/// > if and only if the declaration's scope includes that point.
///
/// So enumeration takes one of these rather than an offset, and reads as that
/// sentence backwards. §6.5's *occurrence* is the other word for a caret and the
/// wrong one: an occurrence is of something written, and at an empty prefix
/// nothing is.
pub(crate) struct Point<'a> {
    /// §6.6.1 is a relation between a declaration and the place reaching for it,
    /// so completion is already one end of it and reuses that end whole.
    at: Site<'a>,
    context: Context,
    prefix: &'a str,
    replace: OffsetSpan,
}

/// §6.5.1 classifies a name by the context it is written in. Only one
/// distinction is load-bearing yet: everything after a `.` is a member of the
/// receiver (§6.5.6.2), and offering names in scope there is actively wrong.
/// Every other position is §6.5.2's *AmbiguousName*, where offering types,
/// variables and methods together is the spec's own answer rather than a
/// simplification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Context {
    Qualified,
    Unqualified,
}

impl<'a> Point<'a> {
    pub(crate) fn at(
        source: &'a jvm::model::Source,
        file: &'a model::File,
        offset: Offset,
        contents: &'a str,
    ) -> Option<Point<'a>> {
        let before = contents.get(..offset.0)?;
        let start = prefix_start(before);

        Some(Point {
            at: Site {
                source,
                file,
                scope: enclosing_scope(file, offset)?,
            },
            context: context_before(&before[..start]),
            prefix: &before[start..],
            replace: OffsetSpan {
                start: Offset(start),
                end: offset,
            },
        })
    }
}

/// The stages of `resolve_type_candidates`, in the same order, each turned
/// around: instead of asking whether a stage spells one name, ask what it
/// spells at all.
///
/// `or_stage` stops at the first stage with a valid candidate; completion
/// cannot, because every stage contributes names. The same rule applies per
/// name instead — a name an earlier stage produced is not looked for in a later
/// one — which is what keeps this and resolution answering alike.
pub(crate) fn complete(
    point: &Point,
    query: &Query,
    revision: Revision,
) -> Vec<CompletionItem<jvm::model::Source>> {
    if point.context == Context::Qualified {
        return Vec::new();
    }

    let mut items = types_in_lexical_scopes(point, revision);
    push_imports(&mut items, point, revision);

    // The two stages that name types nobody wrote down. Each name is enumerated
    // cheaply and handed back to the resolution stage that owns it, and only
    // what that stage calls valid is offered — a type this file cannot see is
    // evidence for a diagnostic, not a suggestion.
    for (name, candidates) in found_names(point, query) {
        push_resolved(&mut items, point, query, revision, &name, candidates);
    }

    items
}

/// Stage 2, §7.5.1. A single-type import introduces its last segment, and that
/// segment is already in the model — this asks nothing of the lake.
///
/// Nor does it check that the import resolves, which is the one place
/// completion deliberately offers a name resolution rejects. An import is not a
/// name we propose; it is one the user wrote, sitting in the buffer, already
/// carrying its own squiggle when it is wrong. Declining to finish typing it
/// protects nobody and reads as a bug.
fn push_imports(
    items: &mut Vec<CompletionItem<jvm::model::Source>>,
    point: &Point,
    revision: Revision,
) {
    for import in &point.at.file.imports {
        if import.kind != model::ImportKind::Type {
            continue;
        }
        let Some(name) = import.name.segments().last() else {
            continue;
        };
        if !name.text.starts_with(point.prefix) || items.iter().any(|i| i.label == name.text) {
            continue;
        }

        let dotted = import.name.dotted();
        items.push(CompletionItem {
            label: name.text.clone(),
            kind: CompletionItemKind::Class,
            detail: Some(dotted.clone()),
            replace: point.replace,
            handle: Some(Handle {
                source: point.at.source.clone(),
                revision,
                payload: dotted,
            }),
        });
    }
}

/// Stages 3 and 4, each as the names it can spell paired with what resolution
/// makes of them.
fn found_names<'a>(
    point: &'a Point,
    query: &'a Query,
) -> impl Iterator<Item = (String, ResolutionCandidates)> + 'a {
    let package = point
        .at
        .file
        .package
        .as_ref()
        .map(model::Name::dotted)
        .unwrap_or_default();

    // Stage 3, §6.3: the top-level types of this compilation unit's package.
    let same_package = query
        .top_level_names_in_package(&package)
        .into_iter()
        .map(move |name| {
            let candidates = candidates_from_same_package(&identifier(&name), &point.at, query);
            (name, candidates)
        });

    // Stage 4, §7.3: `java.lang` is imported on demand into every compilation
    // unit, so it is stage 3 with the package fixed.
    let java_lang = query
        .top_level_names_in_package("java.lang")
        .into_iter()
        .map(move |name| {
            let candidates = candidates_from_java_lang(&identifier(&name), &point.at, query);
            (name, candidates)
        });

    same_package.chain(java_lang)
}

fn push_resolved(
    items: &mut Vec<CompletionItem<jvm::model::Source>>,
    point: &Point,
    query: &Query,
    revision: Revision,
    name: &str,
    candidates: ResolutionCandidates,
) {
    if !name.starts_with(point.prefix) {
        return;
    }
    // §6.4.1 read per name: an earlier stage already won this spelling.
    if items.iter().any(|item| item.label == name) {
        return;
    }
    // Ambiguity survives resolution and does not survive into the list: two
    // declarations of one name are still one row, because the user is picking a
    // name. Which of them it turns out to be is a diagnostic's job once the name
    // is written.
    let Some(target) = candidates.valid().first() else {
        return;
    };

    items.push(CompletionItem {
        label: name.to_string(),
        kind: CompletionItemKind::Class,
        detail: target_label(target, query),
        replace: point.replace,
        handle: target_handle(target, query, revision),
    });
}

/// A `TypeRef`-shaped identifier for a name we are asking about rather than one
/// we read. The span is the caret's own: §6.3's rule that a local is in scope
/// only after its declarator is asked against where we are standing.
fn identifier(text: &str) -> model::Identifier {
    model::Identifier {
        text: text.to_string(),
        span: OffsetSpan {
            start: Offset(0),
            end: Offset(0),
        },
    }
}

/// The dotted name to show beside the label. A file we parsed knows its own
/// (`p.Outer.Inner`); anything else is a binary name and already is one.
fn target_label(target: &TypeTarget, query: &Query) -> Option<String> {
    match target {
        TypeTarget::Parsed {
            source,
            declaration,
        } => query.model_of(source)?.declaration_label(*declaration),
        TypeTarget::Compiled { fqn, .. } => Some(fqn.to_string()),
    }
}

/// Every one of these stages reaches a type nameable from another file, which
/// is exactly the condition for having a handle at all.
fn target_handle(
    target: &TypeTarget,
    query: &Query,
    revision: Revision,
) -> Option<Handle<jvm::model::Source>> {
    Some(Handle {
        source: target.source().clone(),
        revision,
        payload: target_label(target, query)?,
    })
}

/// The inverse of `candidates_from_lexical_scopes`. That one asks each scope
/// whether it spells one name; this one asks what it spells at all.
///
/// §6.4.1 makes nearest-first right in both directions: a type declaration
/// shadows every other type of that name in scope where it occurs, so the first
/// scope to offer a name is the one that owns it.
fn types_in_lexical_scopes(
    point: &Point,
    revision: Revision,
) -> Vec<CompletionItem<jvm::model::Source>> {
    let file = point.at.file;
    let mut items: Vec<CompletionItem<jvm::model::Source>> = Vec::new();

    for (_, scope) in file.iter_scope_chain(point.at.scope) {
        for declaration_id in scope.declarations.iter().copied() {
            let declaration = &file.declarations[declaration_id.0];
            let kind = match declaration {
                model::Declaration::Type(declaration) => type_kind(declaration.kind),
                model::Declaration::TypeParameter(_) => CompletionItemKind::TypeParameter,
                _ => continue,
            };
            let Some(name) = declaration.name() else {
                continue;
            };
            if !name.text.starts_with(point.prefix) {
                continue;
            }
            if items.iter().any(|item| item.label == name.text) {
                continue;
            }

            items.push(CompletionItem {
                label: name.text.clone(),
                kind,
                detail: file.declaration_label(declaration_id),
                replace: point.replace,
                handle: handle_for(point.at.source, file, declaration_id, revision),
            });
        }
    }

    items
}

/// `None` means one thing: this declaration's identity is file-local.
///
/// A type parameter is never nameable from elsewhere. Neither is a local or an
/// anonymous class, whose binary names take "a non-empty sequence of digits"
/// (§13.1) that the compiler chooses and no source can reproduce — and §6.3
/// confines them to one block regardless. Nothing that cannot be named from
/// another file needs a handle, because a handle exists to be given away.
fn handle_for(
    source: &jvm::model::Source,
    file: &model::File,
    declaration_id: model::DeclarationId,
    revision: Revision,
) -> Option<Handle<jvm::model::Source>> {
    let model::Declaration::Type(declaration) = &file.declarations[declaration_id.0] else {
        return None;
    };

    let declared_under_a_method = file
        .iter_scope_chain(declaration.declaring_scope)
        .filter_map(|(_, scope)| scope.owner)
        .any(|owner| !matches!(file.declarations[owner.0], model::Declaration::Type(_)));
    if declared_under_a_method {
        return None;
    }

    Some(Handle {
        source: source.clone(),
        revision,
        payload: file.declaration_label(declaration_id)?,
    })
}

/// The innermost scope containing the caret. `iter_containing` is sorted
/// tightest-first and scope spans are in the index, so this is one filter.
fn enclosing_scope(file: &model::File, offset: Offset) -> Option<model::LexicalScopeId> {
    file.position_index
        .iter_containing(offset)
        .into_iter()
        .find_map(|(_, entity)| match entity {
            model::EntityId::Scope(scope) => Some(scope),
            _ => None,
        })
}

/// The identifier characters immediately behind the caret (§3.8). An empty run
/// is a caret that has typed nothing, which offers everything in scope.
fn prefix_start(before: &str) -> usize {
    before
        .char_indices()
        .rev()
        .take_while(|(_, character)| is_identifier_part(*character))
        .last()
        .map(|(index, _)| index)
        .unwrap_or(before.len())
}

/// §3.8's *JavaLetterOrDigit*, as far as we need it: `$` and `_` are letters to
/// Java, and everything else follows Unicode.
fn is_identifier_part(character: char) -> bool {
    character.is_alphanumeric() || character == '_' || character == '$'
}

fn context_before(before_prefix: &str) -> Context {
    match before_prefix.chars().rev().find(|c| !c.is_whitespace()) {
        Some('.') => Context::Qualified,
        _ => Context::Unqualified,
    }
}

fn type_kind(kind: model::TypeKind) -> CompletionItemKind {
    match kind {
        model::TypeKind::Class => CompletionItemKind::Class,
        model::TypeKind::Interface => CompletionItemKind::Interface,
        model::TypeKind::Enum => CompletionItemKind::Enum,
        model::TypeKind::Record => CompletionItemKind::Record,
        model::TypeKind::AnnotationInterface => CompletionItemKind::AnnotationInterface,
    }
}

#[cfg(test)]
mod tests;
