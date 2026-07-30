use std::path::{Path, PathBuf};

use beans::Beans;
use beans_core::analysis::diagnostic::Diagnostics;
use beans_core::model::Offset;
use beans_platform_jvm::model::JvmSource;
use beans_workspace::model::{Selector, Unit, Workspace};

use beans_test_support::markers::{Cursor, strip_markers};

pub fn fixture() -> Fixture {
    Fixture {
        files: Vec::new(),
        units: Vec::new(),
        cursors: Vec::new(),
        analyses: Vec::new(),
    }
}

pub struct Fixture {
    files: Vec<(PathBuf, String)>,
    units: Vec<Unit>,
    cursors: Vec<Cursor>,
    analyses: Vec<Analysis>,
}

struct Analysis {
    file: PathBuf,
    expectations: Vec<Expectation>,
}

enum Mode {
    Normal,
    ExpectedFailure(String),
}

enum Expect {
    Code { code: String },
    NoCode { code: String },
    CodeAt { cursor: String, code: String },
    ResolvesTo { cursor: String, fqn: String },
    DoesNotResolve { cursor: String },
    ResolvesToTypeParam { cursor: String, name: String },
    ResolvesToLocalType { cursor: String, name: String },
    AmbiguousBetween { cursor: String, fqns: Vec<String> },
    OffersImports { cursor: String, fqns: Vec<String> },
}

struct Expectation {
    expect: Expect,
    mode: Mode,
}

impl Fixture {
    pub fn file(mut self, path: &str, source: &str) -> Self {
        let path = PathBuf::from(path);
        let stripped = strip_markers(source, &path);
        for cursor in &stripped.cursors {
            assert!(
                !self.cursors.iter().any(|c| c.name == cursor.name),
                "duplicate cursor {:?} across fixture files",
                cursor.name
            );
        }
        self.cursors.extend(stripped.cursors);
        self.files.push((path, stripped.clean));
        self
    }

    /// Declare a unit owning the trees at `sources`, depending on the units
    /// named by `depends_on`.
    ///
    /// Flat rather than nested (`.unit("a", |unit| ...)`) so that one tree can
    /// be claimed by two units, which is the interesting case a builder shape
    /// would rule out.
    pub fn unit(mut self, id: &str, sources: &[&str], depends_on: &[&str]) -> Self {
        self.units.push(Unit {
            id: id.to_string(),
            sources: sources
                .iter()
                .map(|base| Selector::Tree {
                    base: root().join(base),
                    includes: vec!["**/*.java".to_string()],
                    excludes: Vec::new(),
                    generated: false,
                })
                .collect(),
            depends_on: depends_on.iter().map(|id| (*id).to_string()).collect(),
            classpath: Vec::new(),
            jdk_home: None,
        });
        self
    }

    pub fn analyze(mut self, path: &str) -> Self {
        self.analyses.push(Analysis {
            file: PathBuf::from(path),
            expectations: Vec::new(),
        });
        self
    }

    /// A diagnostic with `code` must be produced by the analysis.
    pub fn expect(self, code: &str) -> Self {
        self.push_expectation(Expect::Code {
            code: code.to_string(),
        })
    }

    /// No diagnostic with `code` may be produced anywhere in the analysis.
    /// The quiet half of a rule: without it, a check that fires on everything
    /// passes every positive expectation.
    pub fn expect_no(self, code: &str) -> Self {
        self.push_expectation(Expect::NoCode {
            code: code.to_string(),
        })
    }

    pub fn expect_at(self, cursor: &str, code: &str) -> Self {
        self.push_expectation(Expect::CodeAt {
            cursor: cursor.to_string(),
            code: code.to_string(),
        })
    }

    /// The type reference at the named cursor must resolve to `fqn`.
    pub fn resolves_to(self, cursor: &str, fqn: &str) -> Self {
        self.push_expectation(Expect::ResolvesTo {
            cursor: cursor.to_string(),
            fqn: fqn.to_string(),
        })
    }

    /// The name at the named cursor must reach nothing at all. The negative of
    /// `resolves_to`, and the only way to state that something is out of reach
    /// while no diagnostic reports it.
    pub fn does_not_resolve(self, cursor: &str) -> Self {
        self.push_expectation(Expect::DoesNotResolve {
            cursor: cursor.to_string(),
        })
    }

    /// The type reference at the named cursor must resolve to the type
    /// parameter `name`, not to any class or interface.
    pub fn resolves_to_type_param(self, cursor: &str, name: &str) -> Self {
        self.push_expectation(Expect::ResolvesToTypeParam {
            cursor: cursor.to_string(),
            name: name.to_string(),
        })
    }

    pub fn resolves_to_local_type(self, cursor: &str, name: &str) -> Self {
        self.push_expectation(Expect::ResolvesToLocalType {
            cursor: cursor.to_string(),
            name: name.to_string(),
        })
    }

    pub fn ambiguous_between(self, cursor: &str, fqns: &[&str]) -> Self {
        self.push_expectation(Expect::AmbiguousBetween {
            cursor: cursor.to_string(),
            fqns: fqns.iter().map(|fqn| (*fqn).to_string()).collect(),
        })
    }

    pub fn offers_imports(self, cursor: &str, fqns: &[&str]) -> Self {
        self.push_expectation(Expect::OffersImports {
            cursor: cursor.to_string(),
            fqns: fqns.iter().map(|fqn| (*fqn).to_string()).collect(),
        })
    }

    fn push_expectation(mut self, expect: Expect) -> Self {
        let analysis = self
            .analyses
            .last_mut()
            .expect("expectations must follow analyze");
        analysis.expectations.push(Expectation {
            expect,
            mode: Mode::Normal,
        });
        self
    }

    /// The engine is expected to miss the previous expectation. Once it
    /// unexpectedly meets it, the run turns red and asks for promotion:
    /// remove the marker.
    pub fn expected_failure(mut self, reason: &str) -> Self {
        let expectation = self
            .analyses
            .last_mut()
            .and_then(|analysis| analysis.expectations.last_mut())
            .expect("expected_failure must follow an expectation");
        expectation.mode = Mode::ExpectedFailure(reason.to_string());
        self
    }

    pub fn run(self) {
        let Fixture {
            files,
            units,
            cursors,
            analyses,
        } = self;

        let mut beans = Beans::new();
        // A fixture that declares no unit means a project we know nothing
        // about, not an empty one that owns no file. The distinction is the
        // difference between every existing test still seeing its files and
        // none of them seeing anything.
        if !units.is_empty() {
            beans.set_workspace(Workspace {
                tool: "fixture".to_string(),
                units,
            });
        }

        for (path, contents) in &files {
            beans.process(jvm_source(path), contents);
        }

        let mut promotable = Vec::new();
        for analysis in analyses {
            let result = beans
                .analyze(&jvm_source(&analysis.file))
                .unwrap_or_else(|| panic!("no analysis for {}", analysis.file.display()));
            for expectation in analysis.expectations {
                let met = match &expectation.expect {
                    Expect::Code { code } => result.diagnostics.iter().any(|d| d.code == code),
                    Expect::NoCode { code } => !result.diagnostics.iter().any(|d| d.code == code),
                    Expect::CodeAt { cursor, code } => {
                        let cursor = find_cursor(&cursors, cursor, &analysis.file);
                        result.diagnostics.iter().any(|diagnostic| {
                            diagnostic.code == code
                                && diagnostic.span.start.0 <= cursor.offset
                                && cursor.offset < diagnostic.span.end.0
                        })
                    }
                    Expect::ResolvesTo { cursor, fqn } => {
                        let cursor = find_cursor(&cursors, cursor, &analysis.file);
                        resolution_labels(&beans, &analysis.file, cursor.offset)
                            .iter()
                            .any(|label| label == fqn)
                    }
                    Expect::DoesNotResolve { cursor } => {
                        let cursor = find_cursor(&cursors, cursor, &analysis.file);
                        resolution_labels(&beans, &analysis.file, cursor.offset).is_empty()
                    }
                    Expect::AmbiguousBetween { cursor, fqns } => {
                        let cursor = find_cursor(&cursors, cursor, &analysis.file);
                        let mut labels = resolution_labels(&beans, &analysis.file, cursor.offset);
                        labels.sort();
                        let mut expected = fqns.clone();
                        expected.sort();
                        labels == expected
                    }
                    // The engine cannot see these declaration kinds yet.
                    Expect::ResolvesToTypeParam { cursor, .. }
                    | Expect::ResolvesToLocalType { cursor, .. }
                    | Expect::OffersImports { cursor, .. } => {
                        find_cursor(&cursors, cursor, &analysis.file);
                        false
                    }
                };
                match (met, expectation.mode) {
                    (true, Mode::Normal) => {}
                    (false, Mode::Normal) => panic!(
                        "{} in {}; engine produced:\n{}",
                        describe(&expectation.expect),
                        analysis.file.display(),
                        render(&result.diagnostics),
                    ),
                    (false, Mode::ExpectedFailure(_)) => {}
                    (true, Mode::ExpectedFailure(reason)) => promotable.push(reason),
                }
            }
        }

        assert!(
            promotable.is_empty(),
            "expected-to-fail expectations unexpectedly passed, promote them:\n{}",
            promotable.join("\n")
        );
    }
}

/// Tests write relative paths, and `Workspace` promises its consumers absolute
/// ones. Everything the engine is told sits under this root; everything a test
/// writes stays relative to it.
fn root() -> &'static Path {
    Path::new("/beans-fixture")
}

fn jvm_source(path: &Path) -> JvmSource {
    JvmSource::SourceFile {
        path: root().join(path),
    }
}

fn resolution_labels(beans: &Beans, file: &Path, offset: usize) -> Vec<String> {
    beans
        .find_declarations_for(&jvm_source(file), Offset(offset))
        .unwrap_or_default()
        .iter()
        .filter_map(|target| beans.declaration_label(&target.source, target.span))
        .collect()
}

fn find_cursor<'a>(cursors: &'a [Cursor], name: &str, file: &Path) -> &'a Cursor {
    let cursor = cursors
        .iter()
        .find(|c| c.name.as_deref() == Some(name))
        .unwrap_or_else(|| panic!("no cursor named {name:?} in the fixture"));
    assert_eq!(
        cursor.file, file,
        "cursor {name:?} lives in another file than the analyzed one"
    );
    cursor
}

fn describe(expect: &Expect) -> String {
    match expect {
        Expect::Code { code } => format!("expected `{code}`"),
        Expect::NoCode { code } => format!("expected no `{code}`"),
        Expect::CodeAt { cursor, code } => {
            format!("expected `{code}` at <cur:{cursor}>")
        }
        Expect::ResolvesTo { cursor, fqn } => {
            format!("expected <cur:{cursor}> to resolve to `{fqn}`")
        }
        Expect::DoesNotResolve { cursor } => {
            format!("expected <cur:{cursor}> to resolve to nothing")
        }
        Expect::ResolvesToTypeParam { cursor, name } => {
            format!("expected <cur:{cursor}> to resolve to type parameter `{name}`")
        }
        Expect::ResolvesToLocalType { cursor, name } => {
            format!("expected <cur:{cursor}> to resolve to local type `{name}`")
        }
        Expect::AmbiguousBetween { cursor, fqns } => format!(
            "expected <cur:{cursor}> to be ambiguous between {}",
            fqns.join(", ")
        ),
        Expect::OffersImports { cursor, fqns } => format!(
            "expected <cur:{cursor}> to offer imports {}",
            fqns.join(", ")
        ),
    }
}

fn render(diagnostics: &[Diagnostics]) -> String {
    if diagnostics.is_empty() {
        return "  (no diagnostics)".to_string();
    }
    diagnostics
        .iter()
        .map(|d| {
            format!(
                "  {} @ {}..{}: {}",
                d.code, d.span.start, d.span.end, d.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expectation_matches_engine_output() {
        fixture()
            .file(
                "com/example/Foo.java",
                "package com.example;\nclass Foo { void m() { <cur:x>x = 1; } }",
            )
            .analyze("com/example/Foo.java")
            .expect_at("x", "cannot-find-symbol")
            .run();
    }

    #[test]
    fn analysis_reads_files_processed_earlier() {
        fixture()
            .file(
                "com/example/Foo.java",
                "package com.example;\nclass Foo { void m() { x = 1; } }",
            )
            .file("com/example/Bar.java", "package com.example;\nclass Bar {}")
            .analyze("com/example/Foo.java")
            .expect("cannot-find-symbol")
            .run();
    }

    #[test]
    fn failing_expected_failure_holds() {
        fixture()
            .file(
                "com/example/Foo.java",
                "package com.example;\nclass Foo { Bar bar; }",
            )
            .analyze("com/example/Foo.java")
            .expect("unresolvable-type")
            .expected_failure("resolution does not exist yet")
            .run();
    }

    #[test]
    fn resolution_expectations_are_checked_against_the_engine() {
        fixture()
            .file(
                "com/example/Foo.java",
                "package com.example;\nclass Foo { <cur:bar>Bar bar; }",
            )
            .file("com/example/Bar.java", "package com.example;\nclass Bar {}")
            .analyze("com/example/Foo.java")
            .resolves_to("bar", "com.example.Bar")
            .run();
    }

    #[test]
    #[should_panic(expected = "unexpectedly passed")]
    fn passing_expected_failure_demands_promotion() {
        fixture()
            .file(
                "com/example/Foo.java",
                "package com.example;\nclass Foo { void m() { x = 1; } }",
            )
            .analyze("com/example/Foo.java")
            .expect("cannot-find-symbol")
            .expected_failure("this passes today, so the harness must turn red")
            .run();
    }

    #[test]
    #[should_panic(expected = "engine produced")]
    fn missed_expectation_reports_findings() {
        fixture()
            .file(
                "com/example/Foo.java",
                "package com.example;\nclass Foo { Bar bar; }",
            )
            .analyze("com/example/Foo.java")
            .expect("no-such-code")
            .run();
    }
}
