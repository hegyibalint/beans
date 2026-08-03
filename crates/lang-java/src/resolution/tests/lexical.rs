use crate::parser::JavaParser;

use super::*;

/// Resolve `name` where the declaration of `at` looks a type up.
fn lexical(contents: &str, at: &str, name: &str) -> JavaTypeResolution {
    let mut parser = JavaParser::new();
    let file = parser.parse(contents);
    let scope = declaring_scope_of(&file, at);

    resolve_type_from_lexical_scopes(&identifier(name), &source("Test.java"), &file, scope)
}

fn resolves_to(contents: &str, at: &str, name: &str) -> bool {
    matches!(
        lexical(contents, at, name),
        JavaTypeResolution::Resolved(..)
    )
}

#[test]
fn prefers_the_innermost_scope() {
    let mut parser = JavaParser::new();
    let file = parser.parse("class Outer { class X {} class Inner { class X {} } }");
    let outer = file.top_level_declarations[0];
    let outer_scope = type_declaration(&file, outer).body_scope;
    let inner = type_in_scope(&file, outer_scope, "Inner");
    let inner_scope = type_declaration(&file, inner).body_scope;
    let inner_x = type_in_scope(&file, inner_scope, "X");
    let source = source("Outer.java");

    assert_eq!(
        resolve_type_from_lexical_scopes(&identifier("X"), &source, &file, inner_scope),
        JavaTypeResolution::Resolved(JavaTypeTarget::Java {
            source,
            declaration: inner_x,
        })
    );
}

#[test]
fn continues_to_the_parent_scope() {
    let mut parser = JavaParser::new();
    let file = parser.parse("class Outer { class X {} class Inner {} }");
    let outer = file.top_level_declarations[0];
    let outer_scope = type_declaration(&file, outer).body_scope;
    let outer_x = type_in_scope(&file, outer_scope, "X");
    let inner = type_in_scope(&file, outer_scope, "Inner");
    let inner_scope = type_declaration(&file, inner).body_scope;
    let source = source("Outer.java");

    assert_eq!(
        resolve_type_from_lexical_scopes(&identifier("X"), &source, &file, inner_scope),
        JavaTypeResolution::Resolved(JavaTypeTarget::Java {
            source,
            declaration: outer_x,
        })
    );
}

/// §6.3: a member type is in scope throughout the body that declares it, so a
/// reference above the declaration reaches it just as one below does. A resolver
/// that walked declarations in source order would get this wrong.
#[test]
fn a_member_type_is_in_scope_before_it_is_declared() {
    assert!(resolves_to(
        "class Outer { Inner field; class Inner {} }",
        "field",
        "Inner"
    ));
}

/// The same for two top level types of one file, which share the compilation
/// unit scope.
#[test]
fn a_top_level_type_is_in_scope_before_it_is_declared() {
    assert!(resolves_to(
        "class Test { Later field; } class Later {}",
        "field",
        "Later"
    ));
}

/// §6.3 gives a local class a scope that starts at its own declaration and ends
/// with the block, which is narrower than a member type in both directions. We
/// get the far end right and the near end wrong.
///
/// The assertion here is the wrong answer, recorded on purpose so that it turns
/// red the day resolution is fixed; that is the signal to flip it back to
/// `Unresolved`. §6.3 says the scope is "the rest of the immediately enclosing
/// block", and a reference above the declaration is not in the rest of it. The
/// scope span covers the whole block, so position within the block is never
/// consulted.
///
/// The acceptance test meant to catch this cannot. `declaration_label` stops
/// walking at a scope owned by a method (`model.rs`), so a local `Local` inside
/// `Test.m()` is spelled `p.Local`, exactly like a top level `p.Local`, and
/// `resolves_to` cannot tell the two apart.
#[test]
fn a_local_type_is_wrongly_in_scope_before_it_is_declared() {
    assert!(matches!(
        lexical(
            "class Test { void m() { Local before; class Local {} } }",
            "before",
            "Local"
        ),
        JavaTypeResolution::Resolved(..)
    ));
}

#[test]
fn a_local_type_is_not_in_scope_after_its_block() {
    assert_eq!(
        lexical(
            "class Test { void m() { { class Local {} } Local after; } }",
            "after",
            "Local"
        ),
        JavaTypeResolution::Unresolved {
            invalid_candidates: Vec::new(),
        }
    );
}
