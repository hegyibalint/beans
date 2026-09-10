use crate::query::JavaTypeEntry;
use beans_core_model::source::Source;
use beans_lang_java_model::declarations::Declaration;

#[test]
fn an_empty_member_path_returns_the_starting_target_unchanged() {
    let file = crate::lower_into("class Outer {}");
    let source = Source::SourceFile {
        path: "Outer.java".into(),
    };
    let entry = file.iter_declarations().next().unwrap();
    let Declaration::Type(declaration) = entry.declaration else {
        panic!("expected a type declaration");
    };
    let target = JavaTypeEntry {
        source: &source,
        file: &file,
        declaration_index: entry.declaration_index,
        declaration,
    };

    let resolved = target.resolve_member_path(&[]).unwrap();

    assert!(std::ptr::eq(resolved.source, target.source));
    assert!(std::ptr::eq(resolved.file, target.file));
    assert!(std::ptr::eq(resolved.declaration, target.declaration));
    assert_eq!(resolved.declaration_index, target.declaration_index);
}
