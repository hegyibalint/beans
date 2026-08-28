use beans_core::language::{CompletionItem, CompletionItemKind, Handle};
use beans_core::model::{Offset, OffsetSpan};
use beans_core::storage::Revision;
use beans_platform_jvm as jvm;

use crate::accessibility::Site;
use crate::model;
use crate::query::Query;
use crate::resolution::{
    InScopeMethod, InScopeType, InScopeVariable, TypeTarget, Wanted, first_stage_that_answers,
    methods_in_scope, types_in_scope, variables_in_scope,
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

/// The names in scope at `point`, one row each.
///
/// Resolution and completion ask the same enumeration two different questions:
/// `resolve_type_name` filters `types_in_scope` by one name, this filters it by
/// a prefix. Both then hand what they kept to `first_stage_that_answers`, so
/// §6.4.1's order is applied by one function over one chain and neither side can
/// drift from the other.
pub(crate) fn complete(
    point: &Point,
    query: &Query,
    revision: Revision,
) -> Vec<CompletionItem<jvm::model::Source>> {
    if point.context == Context::Qualified {
        return Vec::new();
    }

    let mut items = Vec::new();
    let in_scope = types_in_scope(&point.at, query, Wanted::StartingWith(point.prefix));
    for (name, candidates) in by_name(in_scope) {
        // The whole chain for one spelling, settled the way resolution settles
        // it. A type this file cannot see is evidence for a diagnostic rather
        // than a suggestion, so only the valid half is offered.
        let answered = first_stage_that_answers(candidates.into_iter(), &point.at, query);
        let Some(target) = answered.valid().first() else {
            continue;
        };

        items.push(CompletionItem::plain(
            name,
            kind_of(target, query),
            target_label(target, query),
            point.replace,
            handle_for(target, query, revision),
        ));
    }

    push_unplaced_imports(&mut items, point, revision);
    push_variables(&mut items, point);
    push_methods(&mut items, point);
    items
}

/// §6.5.6's namespace. Nothing here is classified: a declaration reached through
/// your own scope chain is inside the compilation unit you are standing in, so
/// §6.6.1 has nothing to ask.
///
/// No handle. A local, a parameter and a type parameter have no binary name and
/// never will; a field does, but naming one needs the enclosing type and that is
/// the same identity question methods have.
fn push_variables(items: &mut Vec<CompletionItem<jvm::model::Source>>, point: &Point) {
    let file = point.at.file;
    let mut depth_of: Vec<(String, usize)> = Vec::new();

    for variable in variables_in_scope(
        file,
        point.at.scope,
        point.replace.end,
        Wanted::StartingWith(point.prefix),
    ) {
        // §6.4.1 per name: a nearer declaration takes the spelling outright.
        if let Some((_, won)) = depth_of.iter().find(|(name, _)| *name == variable.name) {
            if *won != variable.depth {
                continue;
            }
        }
        depth_of.push((variable.name.clone(), variable.depth));

        let declaration = &file.declarations[variable.declaration.0];
        items.push(CompletionItem::plain(
            variable.name,
            match declaration {
                model::Declaration::Parameter(_) => CompletionItemKind::Parameter,
                model::Declaration::Field(_) => CompletionItemKind::Field,
                _ => CompletionItemKind::Variable,
            },
            written_type(declaration),
            point.replace,
            None,
        ));
    }
}

/// §6.5.7's namespace, for a call with no receiver.
///
/// One row per declaration rather than per name, unlike every other namespace
/// here. A user completing a type is choosing a name; a user completing a call
/// is choosing between signatures, and §15.12.2 would need arguments nobody has
/// typed yet to pick for them.
fn push_methods(items: &mut Vec<CompletionItem<jvm::model::Source>>, point: &Point) {
    let file = point.at.file;

    for method in methods_in_scope(file, point.at.scope, Wanted::StartingWith(point.prefix)) {
        items.push(CompletionItem {
            label: format!("{}({})", method.name, parameters(file, method.declaration)),
            insert: method.name,
            kind: CompletionItemKind::Method,
            detail: returns(file, method.declaration),
            replace: point.replace,
            handle: None,
        });
    }
}

/// A declaration's type as it is written, not as it resolves. `String s` reads
/// `String` whether or not anything answers that name, which is what an editor
/// shows and costs nothing to produce.
fn written_type(declaration: &model::Declaration) -> Option<String> {
    Some(declaration.type_ref()?.ty.to_string())
}

/// `int, String` — the parameter types beside the method name, which is what a
/// row carries.
///
/// Types and not names, though a Java source declaration has both. A compiled
/// method usually has only the types: JVMS §4.7.24 makes `MethodParameters`
/// optional, and javac writes it only under `-parameters`, so most of a jar and
/// much of a runtime image carries no parameter name at all. Showing names
/// where we happen to have them would format one list two ways depending on
/// where each method came from.
///
/// §8.4.2 agrees about which half matters: a signature is the name and the
/// parameter types, and what a parameter is called is not part of it.
fn parameters(file: &model::File, declaration: model::DeclarationId) -> String {
    let model::Declaration::Method(method) = &file.declarations[declaration.0] else {
        return String::new();
    };

    method
        .parameters
        .iter()
        .map(|parameter| {
            written_type(&file.declarations[parameter.0]).unwrap_or_else(|| "?".to_string())
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// What the method hands back, shown to the side of the row. §8.4.5 makes a
/// missing *Result* impossible in legal source, so `None` is a recovered parse
/// rather than a void method — void says so itself.
fn returns(file: &model::File, declaration: model::DeclarationId) -> Option<String> {
    let model::Declaration::Method(method) = &file.declarations[declaration.0] else {
        return None;
    };

    Some(method.return_type.as_ref()?.ty.to_string())
}

/// The chain grouped by spelling, keeping the order stages produced them in so
/// the fold still sees §6.4.1's ranking.
fn by_name(candidates: impl Iterator<Item = InScopeType>) -> Vec<(String, Vec<InScopeType>)> {
    let mut grouped: Vec<(String, Vec<InScopeType>)> = Vec::new();

    for candidate in candidates {
        match grouped.iter_mut().find(|(name, _)| *name == candidate.name) {
            Some((_, group)) => group.push(candidate),
            None => grouped.push((candidate.name.clone(), vec![candidate])),
        }
    }

    grouped
}

/// §7.5.1's names that the chain could not place.
///
/// This is the one place completion outlives resolution. An import is not a name
/// we propose; it is one the user wrote, sitting in the buffer and already
/// carrying its own squiggle when it is wrong, so declining to finish typing it
/// protects nobody. What it *means* is still resolution's to say — anything the
/// chain placed is already offered above, with the winner's own label — and the
/// import's spelling is the fallback for when nothing answers at all.
fn push_unplaced_imports(
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
        items.push(CompletionItem::plain(
            name.text.clone(),
            CompletionItemKind::Class,
            Some(dotted.clone()),
            point.replace,
            Some(Handle {
                source: point.at.source.clone(),
                revision,
                payload: dotted,
            }),
        ));
    }
}

/// The dotted name to show beside a label. A file we parsed knows its own
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

/// `None` means one thing: this declaration's identity is file-local.
///
/// A type parameter is never nameable from elsewhere. Neither is a local or an
/// anonymous class, whose binary names take "a non-empty sequence of digits"
/// (§13.1) that the compiler chooses and no source can reproduce — and §6.3
/// confines them to one block regardless. Nothing that cannot be named from
/// another file needs a handle, because a handle exists to be given away.
fn handle_for(
    target: &TypeTarget,
    query: &Query,
    revision: Revision,
) -> Option<Handle<jvm::model::Source>> {
    let TypeTarget::Parsed {
        source,
        declaration,
    } = target
    else {
        return Some(Handle {
            source: target.source().clone(),
            revision,
            payload: target_label(target, query)?,
        });
    };

    let file = query.model_of(source)?;
    let model::Declaration::Type(type_declaration) = &file.declarations[declaration.0] else {
        return None;
    };
    let declared_under_a_method = file
        .iter_scope_chain(type_declaration.declaring_scope)
        .filter_map(|(_, scope)| scope.owner)
        .any(|owner| !matches!(file.declarations[owner.0], model::Declaration::Type(_)));
    if declared_under_a_method {
        return None;
    }

    Some(Handle {
        source: source.clone(),
        revision,
        payload: target_label(target, query)?,
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

/// What a row's icon says. A file we parsed carries the source keyword; the
/// lake carries the projection of it, and neither is more authoritative than
/// the other about a type it holds.
fn kind_of(target: &TypeTarget, query: &Query) -> CompletionItemKind {
    let TypeTarget::Parsed {
        source,
        declaration,
    } = target
    else {
        return CompletionItemKind::Class;
    };
    let Some(file) = query.model_of(source) else {
        return CompletionItemKind::Class;
    };
    match &file.declarations[declaration.0] {
        model::Declaration::Type(declaration) => type_kind(declaration.kind),
        model::Declaration::TypeParameter(_) => CompletionItemKind::TypeParameter,
        _ => CompletionItemKind::Class,
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
