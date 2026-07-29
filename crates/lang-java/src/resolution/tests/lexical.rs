use crate::parser::JavaParser;

use super::*;

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
        resolve_lexical_type_name(&identifier("X"), &source, &file, inner_scope),
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
        resolve_lexical_type_name(&identifier("X"), &source, &file, inner_scope),
        JavaTypeResolution::Resolved(JavaTypeTarget::Java {
            source,
            declaration: outer_x,
        })
    );
}
