//! Integration test for completion: drive `Language` through its public API and
//! assert that the names in scope come back. The cases live with the code that
//! decides them, in `completion/tests/`; this establishes only that the answer
//! survives the crate boundary.

use std::path::PathBuf;

use beans_core::language::{Language, LanguageProcessing};
use beans_core::model::Offset;
use beans_core::storage::Revision;
use beans_lang_java as java;
use beans_platform_jvm as jvm;
use beans_test_support::markers::strip_markers;

#[test]
fn types_in_scope_reach_the_public_api() {
    let path = PathBuf::from("app/p/Test.java");
    let stripped = strip_markers(
        "package p;
class Test {
    class Inner {}
    void m() {
        <cur>
    }
}
",
        &path,
    );

    let source = jvm::model::Source::SourceFile { path };
    let mut language = java::Language::new();
    let mut platform = jvm::Platform::new();
    let mut revision = Revision::default();

    let at = revision.bump();
    language.process(source.clone(), at, &mut platform, &stripped.clean);

    let items = language
        .complete_at(
            &source,
            Offset(stripped.cursors[0].offset),
            at,
            &platform,
            &stripped.clean,
        )
        .expect("a parsed Java file completes");

    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert_eq!(labels, ["Inner", "Test"]);
}
