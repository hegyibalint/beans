//! Black-box acceptance tests: drive `LanguageJava` through its public API and
//! assert the type it delivers when resolving the reference under a `<cur>`.
//! Markers place the caret, so the test never counts bytes, and the parser and
//! position-index internals stay private — as they should.

use std::path::{Path, PathBuf};

use beans_core::language::{Language, LanguageProcessing};
use beans_core::model::Offset;
use beans_core::storage::Revision;
use beans_lang_java::LanguageJava;
use beans_platform_jvm::PlatformJvm;
use beans_platform_jvm::model::JvmSource;
use beans_test_support::markers::strip_markers;

/// Loads each `(path, contents)` into a fresh language, then returns the type
/// labels resolved at `caret` in `query`. Mirrors what `Beans` does: one bumped
/// revision per file, queried at the latest.
fn resolved_labels(files: &[(&str, &str)], query: &str, caret: Offset) -> Vec<String> {
    let mut language = LanguageJava::new();
    let mut platform = PlatformJvm::new();
    let mut revision = Revision::default();

    for (path, contents) in files {
        let source = JvmSource::SourceFile {
            path: PathBuf::from(path),
        };
        let at = revision.bump();
        language.process(source, at, &mut platform, contents);
    }

    let query = JvmSource::SourceFile {
        path: PathBuf::from(query),
    };
    language
        .find_declarations_for(&query, caret, revision, &platform)
        .unwrap_or_default()
        .iter()
        .filter_map(|target| language.declaration_label(&target.source, target.span, revision))
        .collect()
}

#[test]
fn resolves_a_cross_file_type_when_the_caret_is_at_its_right_edge() {
    // `<cur>` sits at B's right edge — where clicking the right half of the
    // glyph lands. Resolving there must still deliver the type `p.B`, declared
    // in another file.
    let a = strip_markers(
        "package p; class A { B<cur> field; }",
        Path::new("p/A.java"),
    );
    let caret = Offset(a.cursors[0].offset);

    let labels = resolved_labels(
        &[("p/B.java", "package p; class B {}"), ("p/A.java", &a.clean)],
        "p/A.java",
        caret,
    );

    assert!(labels.contains(&"p.B".to_string()), "resolved to {labels:?}");
}
