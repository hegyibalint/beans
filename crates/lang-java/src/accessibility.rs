use beans_platform_jvm::model::JvmSource;

use crate::model::{
    JavaAccess, JavaAccessLevel, JavaDeclaration, JavaDeclarationId, JavaFile, JavaLexicalScopeId,
    JavaName,
};

/// One end of JLS 26 §6.6.1's question. Accessibility is a relation between a
/// declaration and the place reaching for it, so both ends are described the
/// same way and neither is privileged.
pub struct JavaSite<'a> {
    pub source: &'a JvmSource,
    pub file: &'a JavaFile,
    pub scope: JavaLexicalScopeId,
}

/// §6.6.1, for the levels we can decide today.
///
/// `Protected` answers `true` unconditionally. §6.6.2 grants access to a
/// subclass responsible for the implementation of the object, which needs a
/// type hierarchy we do not have, and a wrong `false` here would put a squiggle
/// on correct code.
pub fn is_accessible(access: Option<JavaAccess>, declared: &JavaSite, from: &JavaSite) -> bool {
    let Some(access) = access else {
        return true;
    };

    match access.level {
        JavaAccessLevel::Public | JavaAccessLevel::Protected => true,
        JavaAccessLevel::Package => package_of(declared.file) == package_of(from.file),
        JavaAccessLevel::Private => reaches_the_same_top_level_type(declared, from),
    }
}

/// §6.6.1: private access is permitted when it "occurs from within the body of
/// the top level class or interface that encloses the declaration". The unit is
/// the whole outermost body, so sibling nested types can see each other, and two
/// top level types in one file cannot.
fn reaches_the_same_top_level_type(declared: &JavaSite, from: &JavaSite) -> bool {
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
fn top_level_type(file: &JavaFile, scope: JavaLexicalScopeId) -> Option<JavaDeclarationId> {
    file.iter_scope_chain(scope)
        .filter_map(|(_, scope)| scope.owner)
        .filter(|owner| matches!(file.declarations[owner.0], JavaDeclaration::Type(_)))
        .last()
}

fn package_of(file: &JavaFile) -> String {
    file.package
        .as_ref()
        .map(JavaName::dotted)
        .unwrap_or_default()
}
