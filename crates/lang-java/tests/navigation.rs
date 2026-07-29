//! Integration tests for go-to-declaration: drive `LanguageJava` through its
//! public API and assert where a reference lands. Markers place both the caret
//! and the declaration we expect it to reach, so the test never counts bytes,
//! and the parser and position-index internals stay private — as they should.

use std::path::PathBuf;

use beans_core::language::{Language, LanguageProcessing, NavigationTarget};
use beans_core::model::Offset;
use beans_core::storage::Revision;
use beans_lang_java::LanguageJava;
use beans_platform_jvm::PlatformJvm;
use beans_platform_jvm::model::JvmSource;
use beans_test_support::markers::{Cursor, strip_markers};

/// Files loaded into a fresh language, one bumped revision each and queried at
/// the latest, which is what `Beans` does. Markers are stripped on the way in,
/// so every position a test names is a cursor rather than an offset.
struct Workspace {
    language: LanguageJava,
    platform: PlatformJvm,
    revision: Revision,
    cursors: Vec<Cursor>,
}

impl Workspace {
    fn load(files: &[(&str, &str)]) -> Workspace {
        let mut language = LanguageJava::new();
        let mut platform = PlatformJvm::new();
        let mut revision = Revision::default();
        let mut cursors = Vec::new();

        for (path, contents) in files {
            let path = PathBuf::from(path);
            let stripped = strip_markers(contents, &path);
            let at = revision.bump();
            language.process(source(&path), at, &mut platform, &stripped.clean);
            cursors.extend(stripped.cursors);
        }

        Workspace {
            language,
            platform,
            revision,
            cursors,
        }
    }

    fn cursor(&self, name: &str) -> &Cursor {
        let mut named = self
            .cursors
            .iter()
            .filter(|cursor| cursor.name.as_deref() == Some(name));
        let cursor = named
            .next()
            .unwrap_or_else(|| panic!("no cursor named `{name}`"));
        assert!(
            named.next().is_none(),
            "more than one cursor named `{name}`"
        );
        cursor
    }

    fn declarations_at(&self, name: &str) -> Vec<NavigationTarget<JvmSource>> {
        let cursor = self.cursor(name);
        self.language
            .find_declarations_for(
                &source(&cursor.file),
                Offset(cursor.offset),
                self.revision,
                &self.platform,
            )
            .unwrap_or_default()
    }

    fn labels_at(&self, name: &str) -> Vec<String> {
        self.declarations_at(name)
            .iter()
            .filter_map(|target| {
                self.language
                    .declaration_label(&target.source, target.span, self.revision)
            })
            .collect()
    }

    /// Asserts the reference at `from` reaches one declaration, the one whose
    /// name begins at `to`.
    fn assert_resolves(&self, from: &str, to: &str) {
        let targets = self.declarations_at(from);
        let declaration = self.cursor(to);

        assert_eq!(targets.len(), 1, "`{from}` reached {targets:?}");
        assert_eq!(
            (&targets[0].source, targets[0].span.start),
            (&source(&declaration.file), Offset(declaration.offset)),
            "`{from}` reached {targets:?}, expected `{to}`"
        );
    }
}

fn source(path: &PathBuf) -> JvmSource {
    JvmSource::SourceFile { path: path.clone() }
}

/// The worked example from PLAN.md, with `B` in a second file so member lookup
/// crosses files.
fn worked_example() -> Workspace {
    Workspace::load(&[
        (
            "A.java",
            "class <cur:A>A {\n    int <cur:a_of_A>a;\n\n    void <cur:b>b(<cur:B_ref>B <cur:c>c) {\n        int <cur:d>d = <cur:c_receiver>c.<cur:a_access>a;\n        <cur:this>this.<cur:this_a>a = <cur:d_use>d;\n        <cur:b_call>b(<cur:c_arg>c);\n    }\n}\n",
        ),
        ("B.java", "class <cur:B>B {\n    int <cur:a_of_B>a;\n}\n"),
    ])
}

#[test]
fn a_bare_name_resolves_to_the_parameter_it_names() {
    let workspace = worked_example();

    workspace.assert_resolves("c_receiver", "c");
    workspace.assert_resolves("c_arg", "c");
}

#[test]
fn a_bare_name_resolves_to_the_local_it_names() {
    worked_example().assert_resolves("d_use", "d");
}

#[test]
fn this_resolves_to_the_enclosing_class() {
    worked_example().assert_resolves("this", "A");
}

#[test]
fn a_field_access_through_this_resolves_to_a_field_of_the_enclosing_class() {
    worked_example().assert_resolves("this_a", "a_of_A");
}

#[test]
fn a_field_access_resolves_to_a_field_of_the_receivers_declared_type() {
    worked_example().assert_resolves("a_access", "a_of_B");
}

#[test]
fn an_unqualified_call_resolves_to_a_method_of_the_enclosing_class() {
    worked_example().assert_resolves("b_call", "b");
}

#[test]
fn a_parameter_type_resolves_to_a_class_in_another_file() {
    worked_example().assert_resolves("B_ref", "B");
}

fn local_class_example() -> Workspace {
    Workspace::load(&[(
        "A.java",
        "class A {\n    int g(int <cur:h>h) {\n        class <cur:local>Local {\n            int <cur:get>get() {\n                return <cur:h_use>h;\n            }\n        }\n        return new <cur:local_use>Local().<cur:get_call>get();\n    }\n}\n",
    )])
}

#[test]
fn a_local_class_body_resolves_a_captured_parameter() {
    local_class_example().assert_resolves("h_use", "h");
}

#[test]
fn a_local_class_name_resolves_to_its_declaration() {
    local_class_example().assert_resolves("local_use", "local");
}

#[test]
fn a_call_on_a_local_class_instance_resolves_to_its_method() {
    local_class_example().assert_resolves("get_call", "get");
}

#[test]
fn a_local_is_not_visible_before_its_declarator() {
    let workspace = Workspace::load(&[(
        "A.java",
        "class A {\n    void m() {\n        <cur:use>x = 1;\n        int x;\n    }\n}\n",
    )]);

    assert!(workspace.declarations_at("use").is_empty());
}

#[test]
fn a_parameter_shadows_a_field() {
    let workspace = Workspace::load(&[(
        "A.java",
        "class A {\n    int x;\n    void m(int <cur:parameter>x) {\n        <cur:use>x = 1;\n    }\n}\n",
    )]);

    workspace.assert_resolves("use", "parameter");
}

#[test]
fn resolves_a_cross_file_type_when_the_caret_is_at_its_right_edge() {
    // The cursor sits at B's right edge — where clicking the right half of the
    // glyph lands. Resolving there must still deliver the type `p.B`, declared
    // in another file.
    let workspace = Workspace::load(&[
        ("p/B.java", "package p; class B {}"),
        ("p/A.java", "package p; class A { B<cur:edge> field; }"),
    ]);

    assert_eq!(workspace.labels_at("edge"), vec!["p.B".to_string()]);
}
