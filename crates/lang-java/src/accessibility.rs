use beans_platform_jvm as jvm;

use crate::model;

/// One end of JLS 26 §6.6.1's question. Accessibility is a relation between a
/// declaration and the place reaching for it, so both ends are described the
/// same way and neither is privileged.
pub struct Site<'a> {
    pub source: &'a jvm::model::Source,
    pub file: &'a model::File,
    pub scope: model::LexicalScopeId,
}

/// §6.6.1, for the levels we can decide today.
///
/// `Protected` answers `true` unconditionally. §6.6.2 grants access to a
/// subclass responsible for the implementation of the object, which needs a
/// type hierarchy we do not have, and a wrong `false` here would put a squiggle
/// on correct code.
pub fn is_accessible(access: Option<model::Access>, declared: &Site, from: &Site) -> bool {
    let Some(access) = access else {
        return true;
    };

    match access.level {
        model::AccessLevel::Public | model::AccessLevel::Protected => true,
        model::AccessLevel::Package => package_of(declared.file) == package_of(from.file),
        model::AccessLevel::Private => reaches_the_same_top_level_type(declared, from),
    }
}

/// §6.6.1 asked of a declaration we hold only a class file of. One end of the
/// relation is unchanged; the other has no `Site` to stand on, so the level
/// and the package that declared it are everything we know about it.
///
/// `None` is every case with no answer rather than a permissive one: a local or
/// anonymous class (§8.1.1), and a class the lake no longer holds. `Protected`
/// answers `true` for the reason above.
pub fn is_compiled_type_accessible(
    access: Option<jvm::model::AccessLevel>,
    declared_package: &str,
    from: &Site,
) -> bool {
    match access {
        None | Some(jvm::model::AccessLevel::Public) | Some(jvm::model::AccessLevel::Protected) => {
            true
        }
        Some(jvm::model::AccessLevel::Package) => declared_package == package_of(from.file),
        // §6.6.1 permits private access only from within the body of the top
        // level class enclosing the declaration, and no Java source is ever
        // inside a class file.
        Some(jvm::model::AccessLevel::Private) => false,
    }
}

/// §6.6.1: private access is permitted when it "occurs from within the body of
/// the top level class or interface that encloses the declaration". The unit is
/// the whole outermost body, so sibling nested types can see each other, and two
/// top level types in one file cannot.
fn reaches_the_same_top_level_type(declared: &Site, from: &Site) -> bool {
    if declared.source != from.source {
        return false;
    }

    match (
        top_level_type(declared.file, declared.scope),
        top_level_type(from.file, from.scope),
    ) {
        (Some(declared), Some(from)) => declared == from,
        _ => false,
    }
}

/// The outermost type enclosing a scope. `iter_scope_chain` runs inside out, so
/// the last type owner it reports is the top level one (§7.6).
fn top_level_type(
    file: &model::File,
    scope: model::LexicalScopeId,
) -> Option<model::DeclarationId> {
    file.iter_scope_chain(scope)
        .filter_map(|(_, scope)| scope.owner)
        .filter(|owner| matches!(file.declarations[owner.0], model::Declaration::Type(_)))
        .last()
}

fn package_of(file: &model::File) -> String {
    file.package
        .as_ref()
        .map(model::Name::dotted)
        .unwrap_or_default()
}
