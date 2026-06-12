//! Fixture test harness — graph-backed.
//!
//! Per ADR-0006 / ADR-0021 the harness queries `beans-core`'s graph and
//! registries directly; the prototype `SymbolTable` path is gone.
//!
//! Per the team-lead's step 4+5 direction, dispatch is per-extension via
//! a `match` on the file's extension, gated by Cargo features. The
//! `Language` trait is no longer used.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use beans_core::completion::CompletionCandidates;
use beans_core::graph::{Graph, NodeId};
use beans_core::{Diagnostic, Fix, SourceEdit};
// `Import` is Java-syntactic data; lives behind the `java` feature.
// Without any language feature the fixture parses markers but doesn't
// resolve cursors, so the imports map is a no-op type alias.
#[cfg(feature = "java")]
use beans_core::languages::java::Import;
#[cfg(not(feature = "java"))]
type Import = std::convert::Infallible;
use beans_core::{Modifier, NodePayload, Registries, SymbolKind};

#[cfg(feature = "java")]
use beans_core::languages::java;

use crate::markers::{strip_markers, CursorPosition};

// --- Assertion types ---

#[derive(Debug)]
enum AssertionKind {
    Kind(SymbolKind),
    Fqn(String),
    Name(String),
    ResolvesTo(String),
    HoverContains(String),
    SignatureReturn(String),
    SignatureParams(Vec<(String, String)>),
    Modifiers(Vec<Modifier>),
    ParentFqn(String),
    ChildrenInclude(Vec<String>),
    ChildrenCount(usize),
}

#[derive(Debug)]
enum TestMode {
    Normal,
    Skip(String),
    ExpectedFailure(String),
}

#[derive(Debug)]
struct PendingAssertion {
    cursor_name: Option<String>,
    checks: Vec<AssertionKind>,
    mode: TestMode,
}

struct PendingCompletion {
    cursor_name: Option<String>,
    check_fn: Box<dyn FnOnce(&CompletionCandidates) + Send>,
    mode: TestMode,
}

struct PendingDiagnostics {
    file: PathBuf,
    check_fn: Box<dyn FnOnce(&Findings<'_>) + Send>,
    mode: TestMode,
}

/// A quick-fix assertion: at the cursor, a [`Fix`] labeled `apply_label`
/// must be offered; applying its edits to the cursor's file must yield
/// text containing every `expected_line_runs` entry as a consecutive
/// run of (trimmed) lines. Declarative rather than closure-based so the
/// harness owns the apply semantics — the same semantics a non-LSP
/// consumer would use.
struct PendingQuickFix {
    cursor_name: Option<String>,
    apply_label: Option<String>,
    expected_line_runs: Vec<Vec<String>>,
    mode: TestMode,
}

/// Findings returned by `.diagnostics(path, ...)`. Wraps the
/// per-file `Vec<Diagnostic>` produced by
/// [`beans_core::compute_diagnostics`] and offers small helpers spec
/// tests typically reach for. Iteration over the underlying slice is
/// available via [`Findings::iter`] for assertions the helpers don't
/// cover.
pub struct Findings<'a> {
    diagnostics: &'a [Diagnostic],
}

impl<'a> Findings<'a> {
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> + '_ {
        self.diagnostics.iter()
    }

    pub fn count(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// True iff some diagnostic was emitted with the given rule code.
    pub fn has_code(&self, code: &str) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.code.as_deref() == Some(code))
    }

    /// Number of diagnostics emitted with the given rule code.
    pub fn count_code(&self, code: &str) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.code.as_deref() == Some(code))
            .count()
    }

    /// True iff some diagnostic with the given rule code is anchored at
    /// `line` (0-indexed, matching tree-sitter).
    pub fn has_code_at_line(&self, code: &str, line: u32) -> bool {
        self.diagnostics.iter().any(|d| {
            d.code.as_deref() == Some(code) && d.location.start_line == line
        })
    }
}

// --- CursorAssert builder ---

/// Builder for resolution assertions at a specific cursor position.
pub struct CursorAssert {
    fixture: Fixture,
    cursor_name: Option<String>,
    checks: Vec<AssertionKind>,
    mode: TestMode,
}

impl CursorAssert {
    pub fn kind(mut self, kind: SymbolKind) -> Self {
        self.checks.push(AssertionKind::Kind(kind));
        self
    }

    pub fn fqn(mut self, fqn: &str) -> Self {
        self.checks.push(AssertionKind::Fqn(fqn.to_string()));
        self
    }

    pub fn name(mut self, name: &str) -> Self {
        self.checks.push(AssertionKind::Name(name.to_string()));
        self
    }

    pub fn resolves_to(mut self, fqn: &str) -> Self {
        self.checks.push(AssertionKind::ResolvesTo(fqn.to_string()));
        self
    }

    pub fn hover_contains(mut self, text: &str) -> Self {
        self.checks.push(AssertionKind::HoverContains(text.to_string()));
        self
    }

    pub fn signature_return(mut self, ret: &str) -> Self {
        self.checks.push(AssertionKind::SignatureReturn(ret.to_string()));
        self
    }

    pub fn signature_params(mut self, params: &[(&str, &str)]) -> Self {
        self.checks.push(AssertionKind::SignatureParams(
            params.iter().map(|(n, t)| (n.to_string(), t.to_string())).collect(),
        ));
        self
    }

    pub fn modifiers(mut self, mods: Vec<Modifier>) -> Self {
        self.checks.push(AssertionKind::Modifiers(mods));
        self
    }

    pub fn parent_fqn(mut self, fqn: &str) -> Self {
        self.checks.push(AssertionKind::ParentFqn(fqn.to_string()));
        self
    }

    pub fn children_include(mut self, names: &[&str]) -> Self {
        self.checks.push(AssertionKind::ChildrenInclude(
            names.iter().map(|n| n.to_string()).collect(),
        ));
        self
    }

    pub fn children_count(mut self, count: usize) -> Self {
        self.checks.push(AssertionKind::ChildrenCount(count));
        self
    }

    pub fn skip(mut self, reason: &str) -> Self {
        self.mode = TestMode::Skip(reason.to_string());
        self
    }

    pub fn expected_failure(mut self, reason: &str) -> Self {
        self.mode = TestMode::ExpectedFailure(reason.to_string());
        self
    }

    /// Start resolving at a different named cursor. Finalizes current assertions.
    pub fn resolve(self, cursor_name: &str) -> CursorAssert {
        self.finalize().resolve(cursor_name)
    }

    /// Start resolving at the anonymous cursor. Finalizes current assertions.
    pub fn resolve_default(self) -> CursorAssert {
        self.finalize().resolve_default()
    }

    // Backward compatibility aliases
    #[doc(hidden)]
    pub fn assert_at(self, cursor_name: &str) -> CursorAssert {
        self.resolve(cursor_name)
    }

    #[doc(hidden)]
    pub fn assert_default(self) -> CursorAssert {
        self.resolve_default()
    }

    /// Execute all assertions.
    pub fn run(self) {
        self.finalize().run();
    }

    fn finalize(self) -> Fixture {
        let mut fixture = self.fixture;
        fixture.assertions.push(PendingAssertion {
            cursor_name: self.cursor_name,
            checks: self.checks,
            mode: self.mode,
        });
        fixture
    }
}

// --- CompletionAssert builder ---

pub struct CompletionAssert {
    fixture: Fixture,
}

impl CompletionAssert {
    pub fn expected_failure(mut self, reason: &str) -> Self {
        if let Some(last) = self.fixture.completions.last_mut() {
            last.mode = TestMode::ExpectedFailure(reason.to_string());
        }
        self
    }

    pub fn resolve(self, cursor_name: &str) -> CursorAssert {
        self.fixture.resolve(cursor_name)
    }

    pub fn resolve_default(self) -> CursorAssert {
        self.fixture.resolve_default()
    }

    pub fn complete(self, cursor_name: &str, check: impl FnOnce(&CompletionCandidates) + Send + 'static) -> CompletionAssert {
        self.fixture.complete(cursor_name, check)
    }

    pub fn complete_default(self, check: impl FnOnce(&CompletionCandidates) + Send + 'static) -> CompletionAssert {
        self.fixture.complete_default(check)
    }

    pub fn diagnostics(
        self,
        file: &str,
        check: impl FnOnce(&Findings<'_>) + Send + 'static,
    ) -> DiagnosticsAssert {
        self.fixture.diagnostics(file, check)
    }

    #[doc(hidden)]
    pub fn assert_at(self, cursor_name: &str) -> CursorAssert {
        self.resolve(cursor_name)
    }

    pub fn run(self) {
        self.fixture.run();
    }
}

/// Builder returned by [`Fixture::diagnostics`]. Allows chaining further
/// assertions or modifying the most recently added diagnostics check.
pub struct DiagnosticsAssert {
    fixture: Fixture,
}

impl DiagnosticsAssert {
    pub fn expected_failure(mut self, reason: &str) -> Self {
        if let Some(last) = self.fixture.diagnostics.last_mut() {
            last.mode = TestMode::ExpectedFailure(reason.to_string());
        }
        self
    }

    pub fn resolve(self, cursor_name: &str) -> CursorAssert {
        self.fixture.resolve(cursor_name)
    }

    pub fn resolve_default(self) -> CursorAssert {
        self.fixture.resolve_default()
    }

    pub fn complete(self, cursor_name: &str, check: impl FnOnce(&CompletionCandidates) + Send + 'static) -> CompletionAssert {
        self.fixture.complete(cursor_name, check)
    }

    pub fn complete_default(self, check: impl FnOnce(&CompletionCandidates) + Send + 'static) -> CompletionAssert {
        self.fixture.complete_default(check)
    }

    pub fn diagnostics(
        self,
        file: &str,
        check: impl FnOnce(&Findings<'_>) + Send + 'static,
    ) -> DiagnosticsAssert {
        self.fixture.diagnostics(file, check)
    }

    pub fn quick_fix(self, cursor_name: &str) -> QuickFixAssert {
        self.fixture.quick_fix(cursor_name)
    }

    pub fn quick_fix_default(self) -> QuickFixAssert {
        self.fixture.quick_fix_default()
    }

    pub fn run(self) {
        self.fixture.run();
    }
}

// --- QuickFixAssert builder ---

/// Builder returned by [`Fixture::quick_fix`] /
/// [`Fixture::quick_fix_default`].
///
/// `.apply(label)` selects which offered fix to apply (by its
/// human-readable label). `.expect_lines(&[...])` asserts the applied
/// file contains the given lines as one consecutive run, compared
/// after trimming each line — fixture indentation is noise, adjacency
/// is the anchor. Call `.expect_lines` more than once for multiple
/// independent anchors.
pub struct QuickFixAssert {
    fixture: Fixture,
}

impl QuickFixAssert {
    pub fn apply(mut self, label: &str) -> Self {
        if let Some(last) = self.fixture.quick_fixes.last_mut() {
            last.apply_label = Some(label.to_string());
        }
        self
    }

    pub fn expect_lines(mut self, lines: &[&str]) -> Self {
        if let Some(last) = self.fixture.quick_fixes.last_mut() {
            last.expected_line_runs
                .push(lines.iter().map(|l| l.to_string()).collect());
        }
        self
    }

    pub fn skip(mut self, reason: &str) -> Self {
        if let Some(last) = self.fixture.quick_fixes.last_mut() {
            last.mode = TestMode::Skip(reason.to_string());
        }
        self
    }

    pub fn expected_failure(mut self, reason: &str) -> Self {
        if let Some(last) = self.fixture.quick_fixes.last_mut() {
            last.mode = TestMode::ExpectedFailure(reason.to_string());
        }
        self
    }

    pub fn quick_fix(self, cursor_name: &str) -> QuickFixAssert {
        self.fixture.quick_fix(cursor_name)
    }

    pub fn quick_fix_default(self) -> QuickFixAssert {
        self.fixture.quick_fix_default()
    }

    pub fn diagnostics(
        self,
        file: &str,
        check: impl FnOnce(&Findings<'_>) + Send + 'static,
    ) -> DiagnosticsAssert {
        self.fixture.diagnostics(file, check)
    }

    pub fn run(self) {
        self.fixture.run();
    }
}

// --- Fixture builder ---

/// Test fixture builder. Loads source files, strips cursor markers,
/// builds a graph + registries, and runs assertions.
///
/// Per-extension dispatch is controlled by Cargo features on
/// `beans-test-harness`. A fixture can only handle a file whose
/// extension's feature is enabled.
pub struct Fixture {
    files: Vec<(PathBuf, String)>,
    assertions: Vec<PendingAssertion>,
    completions: Vec<PendingCompletion>,
    diagnostics: Vec<PendingDiagnostics>,
    quick_fixes: Vec<PendingQuickFix>,
}

impl Default for Fixture {
    fn default() -> Self {
        Self::new()
    }
}

impl Fixture {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            assertions: Vec::new(),
            completions: Vec::new(),
            diagnostics: Vec::new(),
            quick_fixes: Vec::new(),
        }
    }

    pub fn file(mut self, path: &str, source: &str) -> Self {
        self.files.push((PathBuf::from(path), source.to_string()));
        self
    }

    pub fn resolve(self, cursor_name: &str) -> CursorAssert {
        CursorAssert {
            fixture: self,
            cursor_name: Some(cursor_name.to_string()),
            checks: Vec::new(),
            mode: TestMode::Normal,
        }
    }

    pub fn resolve_default(self) -> CursorAssert {
        CursorAssert {
            fixture: self,
            cursor_name: None,
            checks: Vec::new(),
            mode: TestMode::Normal,
        }
    }

    pub fn complete(mut self, cursor_name: &str, check: impl FnOnce(&CompletionCandidates) + Send + 'static) -> CompletionAssert {
        self.completions.push(PendingCompletion {
            cursor_name: Some(cursor_name.to_string()),
            check_fn: Box::new(check),
            mode: TestMode::Normal,
        });
        CompletionAssert { fixture: self }
    }

    pub fn complete_default(mut self, check: impl FnOnce(&CompletionCandidates) + Send + 'static) -> CompletionAssert {
        self.completions.push(PendingCompletion {
            cursor_name: None,
            check_fn: Box::new(check),
            mode: TestMode::Normal,
        });
        CompletionAssert { fixture: self }
    }

    /// Run diagnostic rules over `file` and pass the resulting findings
    /// to `check`. Per ADR-0029 diagnostics dispatch by file extension
    /// to the per-language `diagnostics::rules()` function.
    pub fn diagnostics(
        mut self,
        file: &str,
        check: impl FnOnce(&Findings<'_>) + Send + 'static,
    ) -> DiagnosticsAssert {
        self.diagnostics.push(PendingDiagnostics {
            file: PathBuf::from(file),
            check_fn: Box::new(check),
            mode: TestMode::Normal,
        });
        DiagnosticsAssert { fixture: self }
    }

    /// Request quick fixes at the named cursor, which must sit inside
    /// an identifier with an offerable fix (e.g. an unresolved type
    /// use). Chain `.apply(label)` to pick the fix and
    /// `.expect_lines(&[...])` to assert on the applied result.
    pub fn quick_fix(mut self, cursor_name: &str) -> QuickFixAssert {
        self.quick_fixes.push(PendingQuickFix {
            cursor_name: Some(cursor_name.to_string()),
            apply_label: None,
            expected_line_runs: Vec::new(),
            mode: TestMode::Normal,
        });
        QuickFixAssert { fixture: self }
    }

    /// Request quick fixes at the anonymous `<cur>` cursor.
    pub fn quick_fix_default(mut self) -> QuickFixAssert {
        self.quick_fixes.push(PendingQuickFix {
            cursor_name: None,
            apply_label: None,
            expected_line_runs: Vec::new(),
            mode: TestMode::Normal,
        });
        QuickFixAssert { fixture: self }
    }

    #[doc(hidden)]
    pub fn assert_at(self, cursor_name: &str) -> CursorAssert {
        self.resolve(cursor_name)
    }

    #[doc(hidden)]
    pub fn assert_default(self) -> CursorAssert {
        self.resolve_default()
    }

    pub fn run(self) {
        // 1. Strip markers from all files.
        let mut all_cursors: Vec<CursorPosition> = Vec::new();
        let mut stripped_files: Vec<(PathBuf, String)> = Vec::new();

        for (path, source) in &self.files {
            let stripped = strip_markers(source, path);
            all_cursors.extend(stripped.cursors);
            stripped_files.push((path.clone(), stripped.clean));
        }

        // Validate cursor name uniqueness across files.
        let mut seen_names: HashMap<Option<&str>, &Path> = HashMap::new();
        for cursor in &all_cursors {
            let key = cursor.name.as_deref();
            if let Some(existing) = seen_names.get(&key) {
                let name_display = key.unwrap_or("<anonymous>");
                panic!(
                    "duplicate cursor '{}' found in {} and {}",
                    name_display,
                    existing.display(),
                    cursor.file.display()
                );
            }
            seen_names.insert(key, &cursor.file);
        }

        // 2. Parse all files and integrate into one graph + registries.
        // The graph is mutable when any language feature is on (so
        // `integrate` can insert payloads). With every language feature
        // off the harness can still strip markers but doesn't write to
        // the graph.
        #[cfg_attr(not(feature = "java"), allow(unused_mut))]
        let mut graph: Graph<NodePayload> = Graph::new();
        let registries = Registries::new();
        let mut file_imports: HashMap<PathBuf, Vec<Import>> = HashMap::new();
        let mut file_packages: HashMap<PathBuf, String> = HashMap::new();
        let mut file_sources: HashMap<PathBuf, String> = HashMap::new();

        for (path, clean_source) in &stripped_files {
            let parsed = parse_for_extension(path, clean_source);
            file_imports.insert(path.clone(), parsed.imports.clone());
            if !parsed.package.is_empty() {
                file_packages.insert(path.clone(), parsed.package.clone());
            }
            file_sources.insert(path.clone(), clean_source.clone());
            #[cfg(feature = "java")]
            java::integrate(&mut graph, &registries, parsed.into_java());
            #[cfg(not(feature = "java"))]
            drop(parsed);
        }

        // 3. Run resolution assertions.
        let mut skipped = Vec::new();
        let mut expected_failure_passed = Vec::new();

        for assertion in &self.assertions {
            let cursor_display = assertion
                .cursor_name
                .as_deref()
                .unwrap_or("<default>");

            let cursor = all_cursors
                .iter()
                .find(|c| c.name == assertion.cursor_name)
                .unwrap_or_else(|| {
                    panic!("cursor '{}' not found in any file", cursor_display);
                });

            match &assertion.mode {
                TestMode::Skip(reason) => {
                    skipped.push(format!("SKIP [{}]: {}", cursor_display, reason));
                    continue;
                }
                TestMode::ExpectedFailure(reason) => {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        run_checks(
                            &assertion.checks,
                            cursor,
                            &graph,
                            &registries,
                            &file_imports,
                            &file_packages,
                            &file_sources,
                            cursor_display,
                        );
                    }));
                    match result {
                        Ok(()) => {
                            expected_failure_passed.push(format!(
                                "EXPECTED_FAILURE PASSED [{}]: expected failure '{}' but checks passed — promote this test!",
                                cursor_display, reason
                            ));
                        }
                        Err(_) => {
                            // Expected failure — fine
                        }
                    }
                    continue;
                }
                TestMode::Normal => {}
            }

            run_checks(
                &assertion.checks,
                cursor,
                &graph,
                &registries,
                &file_imports,
                &file_packages,
                &file_sources,
                cursor_display,
            );
        }

        // 4. Run completion assertions. Stub: empty until graph-driven
        //    completion lands per backlog #025 / #027.
        for completion in self.completions {
            let cursor_display = completion
                .cursor_name
                .as_deref()
                .unwrap_or("<default>");

            let _cursor = all_cursors
                .iter()
                .find(|c| c.name == completion.cursor_name)
                .unwrap_or_else(|| {
                    panic!("cursor '{}' not found in any file", cursor_display);
                });

            let items = CompletionCandidates::default();

            match &completion.mode {
                TestMode::Skip(reason) => {
                    skipped.push(format!("SKIP completion [{}]: {}", cursor_display, reason));
                    continue;
                }
                TestMode::ExpectedFailure(reason) => {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        (completion.check_fn)(&items);
                    }));
                    match result {
                        Ok(()) => {
                            expected_failure_passed.push(format!(
                                "EXPECTED_FAILURE PASSED [completion {}]: expected failure '{}' but checks passed — promote this test!",
                                cursor_display, reason
                            ));
                        }
                        Err(_) => {}
                    }
                    continue;
                }
                TestMode::Normal => {
                    (completion.check_fn)(&items);
                }
            }
        }

        // 5. Run diagnostic assertions. Per ADR-0029 each diagnostic
        //    pass dispatches by file extension to the per-language
        //    `diagnostics::rules()` function.
        for diag in self.diagnostics {
            let file_display = diag.file.display().to_string();
            let imports = file_imports
                .get(&diag.file)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let diagnostics = beans_core::compute_diagnostics(
                &graph,
                &registries,
                &diag.file,
                imports,
            );
            let findings = Findings {
                diagnostics: &diagnostics,
            };
            match &diag.mode {
                TestMode::Skip(reason) => {
                    skipped.push(format!(
                        "SKIP diagnostics [{}]: {}",
                        file_display, reason
                    ));
                    continue;
                }
                TestMode::ExpectedFailure(reason) => {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        (diag.check_fn)(&findings);
                    }));
                    if result.is_ok() {
                        expected_failure_passed.push(format!(
                            "EXPECTED_FAILURE PASSED [diagnostics {}]: \
                             expected failure '{}' but checks passed — \
                             promote this test!",
                            file_display, reason
                        ));
                    }
                    continue;
                }
                TestMode::Normal => {
                    (diag.check_fn)(&findings);
                }
            }
        }

        // 6. Run quick-fix assertions. Stub: no fix synthesis exists
        //    yet, so the computed list is always empty and every
        //    assertion fails until the Java `missing-import` rule and
        //    its `Fix` synthesis land (mirrors the completion stub in
        //    step 4).
        for qf in self.quick_fixes {
            let cursor_display = qf.cursor_name.as_deref().unwrap_or("<default>");

            let cursor = all_cursors
                .iter()
                .find(|c| c.name == qf.cursor_name)
                .unwrap_or_else(|| {
                    panic!("cursor '{}' not found in any file", cursor_display);
                });

            let fixes: Vec<Fix> = Vec::new();

            match &qf.mode {
                TestMode::Skip(reason) => {
                    skipped.push(format!(
                        "SKIP quick_fix [{}]: {}",
                        cursor_display, reason
                    ));
                    continue;
                }
                TestMode::ExpectedFailure(reason) => {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        run_quick_fix_checks(&qf, &fixes, &cursor.file, &file_sources, cursor_display);
                    }));
                    if result.is_ok() {
                        expected_failure_passed.push(format!(
                            "EXPECTED_FAILURE PASSED [quick_fix {}]: \
                             expected failure '{}' but checks passed — \
                             promote this test!",
                            cursor_display, reason
                        ));
                    }
                    continue;
                }
                TestMode::Normal => {
                    run_quick_fix_checks(&qf, &fixes, &cursor.file, &file_sources, cursor_display);
                }
            }
        }

        if !skipped.is_empty() {
            eprintln!("--- Skipped assertions ---");
            for msg in &skipped {
                eprintln!("  {}", msg);
            }
        }

        if !expected_failure_passed.is_empty() {
            let msg = expected_failure_passed.join("\n");
            panic!("Expected-to-fail assertions unexpectedly passed:\n{}", msg);
        }
    }
}

// ---------------------------------------------------------------------
// Per-extension parse dispatch.
// ---------------------------------------------------------------------

/// Output of a per-extension parse — wraps the language-specific
/// `Parsed*` value plus shared metadata (package, imports). Implemented
/// as an enum gated by language feature so the harness can dispatch
/// without `dyn`.
struct ParsedForFixture {
    package: String,
    imports: Vec<Import>,
    #[cfg(feature = "java")]
    java: Option<java::ParsedJavaFile>,
}

impl ParsedForFixture {
    #[cfg(feature = "java")]
    fn into_java(self) -> java::ParsedJavaFile {
        self.java.expect(
            "ParsedForFixture::into_java called on non-Java parse — fixture dispatch bug",
        )
    }
}

fn parse_for_extension(path: &Path, #[allow(unused_variables)] source: &str) -> ParsedForFixture {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        #[cfg(feature = "java")]
        "java" => {
            let parsed = java::parse_java_to_graph(path, source);
            ParsedForFixture {
                package: parsed.package.clone(),
                imports: parsed.imports.clone(),
                java: Some(parsed),
            }
        }
        _ => panic!(
            "no parser registered for extension '.{}' (file: {}). \
             Enable the matching beans-test-harness feature.",
            ext,
            path.display()
        ),
    }
}

// ---------------------------------------------------------------------
// Resolution + assertion execution against the graph.
// ---------------------------------------------------------------------

/// A view of a node's identity-bearing fields, sufficient for every
/// assertion the fixture supports. Decouples assertions from whether
/// the underlying payload is Java or JVM-projection — the Java side
/// wins when present, since spec tests assert source-level facts.
struct ResolvedView<'a> {
    id: NodeId,
    payload: &'a NodePayload,
    name: String,
    fqn: String,
    kind: SymbolKind,
    modifiers: Vec<Modifier>,
}

impl<'a> ResolvedView<'a> {
    fn from_node(graph: &'a Graph<NodePayload>, id: NodeId) -> Option<Self> {
        let node = graph.get(id)?;
        let payload = &node.payload;
        let (name, fqn, kind, modifiers) = view_fields(payload)?;
        Some(Self {
            id,
            payload,
            name,
            fqn,
            kind,
            modifiers,
        })
    }
}

fn view_fields(payload: &NodePayload) -> Option<(String, String, SymbolKind, Vec<Modifier>)> {
    match payload {
        #[cfg(feature = "java")]
        NodePayload::Java(java_payload) => java_view_fields(java_payload),
        NodePayload::Jvm(jvm_payload) => Some(jvm_view_fields(jvm_payload)),
    }
}

#[cfg(feature = "java")]
fn java_view_fields(
    payload: &beans_core::languages::java::JavaNodePayload,
) -> Option<(String, String, SymbolKind, Vec<Modifier>)> {
    use beans_core::languages::java::{JavaNodePayload, JavaTypeKind};
    let view = match payload {
        JavaNodePayload::Type(n) => {
            let kind = match n.kind {
                JavaTypeKind::Class => SymbolKind::Class,
                JavaTypeKind::Interface => SymbolKind::Interface,
                JavaTypeKind::Enum => SymbolKind::Enum,
                JavaTypeKind::Record => SymbolKind::Record,
                JavaTypeKind::Annotation => SymbolKind::Annotation,
            };
            (
                n.header.name.clone(),
                n.header.fqn.to_string(),
                kind,
                n.header.modifiers.clone(),
            )
        }
        JavaNodePayload::Method(n) => (
            n.header.name.clone(),
            n.header.fqn.to_string(),
            SymbolKind::Method,
            n.header.modifiers.clone(),
        ),
        JavaNodePayload::Constructor(n) => (
            n.header.name.clone(),
            n.header.fqn.to_string(),
            SymbolKind::Constructor,
            n.header.modifiers.clone(),
        ),
        JavaNodePayload::Field(n) => (
            n.header.name.clone(),
            n.header.fqn.to_string(),
            SymbolKind::Field,
            n.header.modifiers.clone(),
        ),
        JavaNodePayload::EnumConstant(n) => (
            n.header.name.clone(),
            n.header.fqn.to_string(),
            // The prototype emitted enum constants as Field and the
            // existing spec tests assert SymbolKind::Field; preserve.
            SymbolKind::Field,
            n.header.modifiers.clone(),
        ),
        JavaNodePayload::AnnotationElement(n) => (
            n.header.name.clone(),
            n.header.fqn.to_string(),
            SymbolKind::Method,
            n.header.modifiers.clone(),
        ),
        JavaNodePayload::Parameter(p) => (
            p.name.clone(),
            String::new(),
            SymbolKind::Parameter,
            Vec::new(),
        ),
        JavaNodePayload::Package(n) => (
            n.header.name.clone(),
            n.header.fqn.to_string(),
            SymbolKind::Package,
            n.header.modifiers.clone(),
        ),
        // Use-site nodes are not declaration views; resolution at a
        // cursor lands on the use site's *target*, not on the use
        // itself. Cursor assertions never see a TypeUse here.
        JavaNodePayload::TypeUse(_) => return None,
        // Imports are location carriers, not declarations (ADR-0029).
        JavaNodePayload::Import(_) => return None,
    };
    Some(view)
}

fn jvm_view_fields(
    payload: &beans_core::jvm::JvmNodePayload,
) -> (String, String, SymbolKind, Vec<Modifier>) {
    use beans_core::jvm::{JvmNodePayload, JvmTypeKind};
    match payload {
        JvmNodePayload::Type(n) => {
            let kind = match n.kind {
                JvmTypeKind::Class => SymbolKind::Class,
                JvmTypeKind::Interface => SymbolKind::Interface,
                JvmTypeKind::Enum => SymbolKind::Enum,
                JvmTypeKind::Record => SymbolKind::Record,
                JvmTypeKind::Annotation => SymbolKind::Annotation,
            };
            (
                n.header.name.clone(),
                n.header.fqn.to_string(),
                kind,
                n.header.modifiers.clone(),
            )
        }
        JvmNodePayload::Method(n) => (
            n.header.name.clone(),
            n.header.fqn.to_string(),
            SymbolKind::Method,
            n.header.modifiers.clone(),
        ),
        JvmNodePayload::Constructor(n) => (
            n.header.name.clone(),
            n.header.fqn.to_string(),
            SymbolKind::Constructor,
            n.header.modifiers.clone(),
        ),
        JvmNodePayload::Field(n) => (
            n.header.name.clone(),
            n.header.fqn.to_string(),
            SymbolKind::Field,
            n.header.modifiers.clone(),
        ),
        JvmNodePayload::EnumConstant(n) => (
            n.header.name.clone(),
            n.header.fqn.to_string(),
            SymbolKind::Field,
            n.header.modifiers.clone(),
        ),
        JvmNodePayload::AnnotationElement(n) => (
            n.header.name.clone(),
            n.header.fqn.to_string(),
            SymbolKind::Method,
            n.header.modifiers.clone(),
        ),
        JvmNodePayload::Parameter(p) => (
            p.name.clone(),
            String::new(),
            SymbolKind::Parameter,
            Vec::new(),
        ),
        JvmNodePayload::Package(n) => (
            n.header.name.clone(),
            n.header.fqn.to_string(),
            SymbolKind::Package,
            n.header.modifiers.clone(),
        ),
    }
}

/// Resolve the word at the cursor through the registries, preferring the
/// language-side node over its JVM projection (spec tests assert
/// source-level facts).
fn resolve_at_cursor<'a>(
    cursor: &CursorPosition,
    graph: &'a Graph<NodePayload>,
    registries: &Registries,
    file_imports: &HashMap<PathBuf, Vec<Import>>,
    file_packages: &HashMap<PathBuf, String>,
    source: &str,
    cursor_display: &str,
) -> ResolvedView<'a> {
    let word = word_at(source, cursor.line, cursor.col, &cursor.file).unwrap_or_else(|| {
        panic!(
            "[{}] no word at cursor position ({}:{} in {})",
            cursor_display, cursor.line, cursor.col, cursor.file.display()
        );
    });

    let imports = file_imports
        .get(&cursor.file)
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    let current_package = file_packages
        .get(&cursor.file)
        .map(|s| s.as_str())
        .unwrap_or("");

    // Per the team-lead's step 4+5 direction the harness dispatches by
    // file extension. Java resolution is the only chain implemented
    // today; when other languages land they'll add their own arms gated
    // by their own features. Without any language feature the harness
    // can still parse markers but won't resolve cursor positions.
    #[cfg(feature = "java")]
    let resolved =
        java::resolve_name(&word, imports, current_package, registries, graph);
    #[cfg(not(feature = "java"))]
    let resolved: Option<NodeId> = {
        let _ = (imports, current_package, registries, graph);
        None
    };
    let id = resolved.unwrap_or_else(|| {
        panic!(
            "[{}] could not resolve '{}' to any symbol",
            cursor_display, word
        );
    });

    ResolvedView::from_node(graph, id).unwrap_or_else(|| {
        panic!(
            "[{}] resolved id {:?} but no view-shaped payload at that node",
            cursor_display, id
        );
    })
}

#[allow(unused_variables)]
fn word_at(source: &str, line: u32, col: u32, file: &Path) -> Option<String> {
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        #[cfg(feature = "java")]
        "java" => java::word_at_position(source, line, col),
        _ => None,
    }
}

// Resolution helpers (FQN chain + simple-name fallback) live in
// `beans_core::languages::java::resolve` so the LSP and the fixture
// share one implementation. Per ADR-0012 / ADR-0020 they return raw
// `NodeId`; LSP-shaped output formatting belongs in `beans-lsp`.

// ---------------------------------------------------------------------
// Hover, signature, parent/children — payload-shape introspection.
// ---------------------------------------------------------------------

fn build_hover(view: &ResolvedView<'_>) -> String {
    use std::fmt::Write;

    // The `EnumConstant` arm is reachable in principle but unreachable
    // today: `view_fields` collapses `JavaNodePayload::EnumConstant`
    // into `SymbolKind::Field` for spec-test stability. Backlog #032
    // tracks whether to surface `EnumConstant` distinctly.
    let kind_str = match view.kind {
        SymbolKind::Class => "class",
        SymbolKind::Interface => "interface",
        SymbolKind::Enum => "enum",
        SymbolKind::Record => "record",
        SymbolKind::Annotation => "@interface",
        SymbolKind::Method => "method",
        SymbolKind::Constructor => "constructor",
        SymbolKind::Field | SymbolKind::EnumConstant => "field",
        SymbolKind::Parameter => "parameter",
        SymbolKind::Package => "package",
    };

    let mut s = String::new();

    match view.payload {
        #[cfg(feature = "java")]
        NodePayload::Java(beans_core::languages::java::JavaNodePayload::Method(m)) => {
            let tp = if m.type_parameters.is_empty() {
                String::new()
            } else {
                let names: Vec<&str> = m.type_parameters.iter().map(|t| t.name.as_str()).collect();
                format!("<{}>", names.join(", "))
            };
            let params: Vec<String> = m
                .parameters
                .iter()
                .map(|p| format!("{} {}", p.param_type, p.name))
                .collect();
            let _ = write!(
                s,
                "```java\n{}{} {}({})\n```\n\n{} `{}`",
                tp,
                m.return_type,
                m.header.name,
                params.join(", "),
                kind_str,
                m.header.fqn
            );
        }
        #[cfg(feature = "java")]
        NodePayload::Java(beans_core::languages::java::JavaNodePayload::Field(f)) => {
            let _ = write!(
                s,
                "```java\n{} {}\n```\n\n{} `{}`",
                f.field_type, f.header.name, kind_str, f.header.fqn
            );
        }
        #[cfg(feature = "java")]
        NodePayload::Java(beans_core::languages::java::JavaNodePayload::Type(t)) => {
            let tp = if t.type_parameters.is_empty() {
                String::new()
            } else {
                let names: Vec<&str> = t.type_parameters.iter().map(|p| p.name.as_str()).collect();
                format!("<{}>", names.join(", "))
            };
            let header = match t.kind {
                beans_core::languages::java::JavaTypeKind::Record => format!("record {}{}", t.header.name, tp),
                _ => format!("{} {}{}", kind_str, t.header.name, tp),
            };
            let _ = write!(s, "```java\n{}\n```\n\n`{}`", header, t.header.fqn);
        }
        _ => {
            let _ = write!(s, "```java\n{} {}\n```\n\n`{}`", kind_str, view.name, view.fqn);
        }
    }

    s
}

fn signature_return_type(view: &ResolvedView<'_>) -> Option<String> {
    match view.payload {
        #[cfg(feature = "java")]
        NodePayload::Java(beans_core::languages::java::JavaNodePayload::Method(m)) => {
            Some(m.return_type.to_string())
        }
        _ => None,
    }
}

fn signature_params(view: &ResolvedView<'_>) -> Option<Vec<(String, String)>> {
    match view.payload {
        #[cfg(feature = "java")]
        NodePayload::Java(beans_core::languages::java::JavaNodePayload::Method(m)) => Some(
            m.parameters
                .iter()
                .map(|p| (p.name.clone(), p.param_type.to_string()))
                .collect(),
        ),
        #[cfg(feature = "java")]
        NodePayload::Java(beans_core::languages::java::JavaNodePayload::Constructor(c)) => Some(
            c.parameters
                .iter()
                .map(|p| (p.name.clone(), p.param_type.to_string()))
                .collect(),
        ),
        _ => None,
    }
}

fn parent_view<'a>(
    view: &ResolvedView<'_>,
    graph: &'a Graph<NodePayload>,
) -> Option<ResolvedView<'a>> {
    let parent_id = graph.get(view.id)?.parent?;
    ResolvedView::from_node(graph, parent_id)
}

/// Collect names of children at one level down, excluding the JVM
/// projection sibling. Java nodes hard-link both their own JVM
/// projection and any source-level children (methods, fields, nested
/// types); spec tests assert the *source-level* children list, so we
/// filter out JVM projections.
fn child_names(view: &ResolvedView<'_>, graph: &Graph<NodePayload>) -> Vec<String> {
    let node = match graph.get(view.id) {
        Some(n) => n,
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    for &child_id in &node.children {
        if let Some(child) = graph.get(child_id) {
            // Skip the JVM projection; per the parser's plan layout the
            // first child of a Java node is its JVM projection sibling.
            if matches!(child.payload, NodePayload::Jvm(_)) {
                continue;
            }
            // The match's arm types only unify when a language feature
            // is on (the Java arm yields `Option<&JavaDeclHeader>`).
            // With every language feature off the only arm is the JVM
            // catch-all, and Rust can't infer the `Option<_>` element
            // type. Annotate the match expression to keep it typed.
            let header = match &child.payload {
                #[cfg(feature = "java")]
                NodePayload::Java(j) => j.header().map(|h| h.name.clone()),
                NodePayload::Jvm(_) => Option::<String>::None,
            };
            if let Some(name) = header {
                out.push(name);
            }
        }
    }
    out
}

fn run_checks(
    checks: &[AssertionKind],
    cursor: &CursorPosition,
    graph: &Graph<NodePayload>,
    registries: &Registries,
    file_imports: &HashMap<PathBuf, Vec<Import>>,
    file_packages: &HashMap<PathBuf, String>,
    file_sources: &HashMap<PathBuf, String>,
    cursor_display: &str,
) {
    let source = file_sources
        .get(&cursor.file)
        .expect("source not found for cursor file");

    for check in checks {
        match check {
            AssertionKind::ResolvesTo(expected_fqn) => {
                let view = resolve_at_cursor(
                    cursor, graph, registries, file_imports, file_packages, source, cursor_display,
                );
                assert_eq!(
                    view.fqn, *expected_fqn,
                    "[{}] resolves_to: expected '{}', got '{}'",
                    cursor_display, expected_fqn, view.fqn
                );
            }
            AssertionKind::Kind(expected_kind) => {
                let view = resolve_at_cursor(
                    cursor, graph, registries, file_imports, file_packages, source, cursor_display,
                );
                assert_eq!(
                    view.kind, *expected_kind,
                    "[{}] kind: expected {:?}, got {:?}",
                    cursor_display, expected_kind, view.kind
                );
            }
            AssertionKind::Fqn(expected_fqn) => {
                let view = resolve_at_cursor(
                    cursor, graph, registries, file_imports, file_packages, source, cursor_display,
                );
                assert_eq!(
                    view.fqn, *expected_fqn,
                    "[{}] fqn: expected '{}', got '{}'",
                    cursor_display, expected_fqn, view.fqn
                );
            }
            AssertionKind::Name(expected_name) => {
                let view = resolve_at_cursor(
                    cursor, graph, registries, file_imports, file_packages, source, cursor_display,
                );
                assert_eq!(
                    view.name, *expected_name,
                    "[{}] name: expected '{}', got '{}'",
                    cursor_display, expected_name, view.name
                );
            }
            AssertionKind::HoverContains(text) => {
                let view = resolve_at_cursor(
                    cursor, graph, registries, file_imports, file_packages, source, cursor_display,
                );
                let hover = build_hover(&view);
                assert!(
                    hover.contains(text.as_str()),
                    "[{}] hover_contains: '{}' not found in hover text:\n{}",
                    cursor_display, text, hover
                );
            }
            AssertionKind::SignatureReturn(expected_ret) => {
                let view = resolve_at_cursor(
                    cursor, graph, registries, file_imports, file_packages, source, cursor_display,
                );
                let return_type = signature_return_type(&view).unwrap_or_else(|| {
                    panic!(
                        "[{}] signature_return: expected Method-shaped view, got kind {:?}",
                        cursor_display, view.kind
                    )
                });
                assert_eq!(
                    return_type, *expected_ret,
                    "[{}] signature_return: expected '{}', got '{}'",
                    cursor_display, expected_ret, return_type
                );
            }
            AssertionKind::SignatureParams(expected_params) => {
                let view = resolve_at_cursor(
                    cursor, graph, registries, file_imports, file_packages, source, cursor_display,
                );
                let params = signature_params(&view).unwrap_or_else(|| {
                    panic!(
                        "[{}] signature_params: expected Method/Constructor view, got kind {:?}",
                        cursor_display, view.kind
                    )
                });
                assert_eq!(
                    params.len(),
                    expected_params.len(),
                    "[{}] signature_params: expected {} params, got {}",
                    cursor_display, expected_params.len(), params.len()
                );
                for (i, (exp_name, exp_type)) in expected_params.iter().enumerate() {
                    assert_eq!(
                        params[i].0, *exp_name,
                        "[{}] param[{}] name: expected '{}', got '{}'",
                        cursor_display, i, exp_name, params[i].0
                    );
                    assert_eq!(
                        params[i].1, *exp_type,
                        "[{}] param[{}] type: expected '{}', got '{}'",
                        cursor_display, i, exp_type, params[i].1
                    );
                }
            }
            AssertionKind::Modifiers(expected_mods) => {
                let view = resolve_at_cursor(
                    cursor, graph, registries, file_imports, file_packages, source, cursor_display,
                );
                for m in expected_mods {
                    assert!(
                        view.modifiers.contains(m),
                        "[{}] modifiers: expected {:?} but symbol has {:?}",
                        cursor_display, m, view.modifiers
                    );
                }
            }
            AssertionKind::ParentFqn(expected_fqn) => {
                let view = resolve_at_cursor(
                    cursor, graph, registries, file_imports, file_packages, source, cursor_display,
                );
                let parent = parent_view(&view, graph).unwrap_or_else(|| {
                    panic!("[{}] parent_fqn: symbol has no parent", cursor_display);
                });
                assert_eq!(
                    parent.fqn, *expected_fqn,
                    "[{}] parent_fqn: expected '{}', got '{}'",
                    cursor_display, expected_fqn, parent.fqn
                );
            }
            AssertionKind::ChildrenInclude(expected_names) => {
                let view = resolve_at_cursor(
                    cursor, graph, registries, file_imports, file_packages, source, cursor_display,
                );
                let names = child_names(&view, graph);
                for name in expected_names {
                    assert!(
                        names.contains(name),
                        "[{}] children_include: '{}' not found among children {:?}",
                        cursor_display, name, names
                    );
                }
            }
            AssertionKind::ChildrenCount(expected_count) => {
                let view = resolve_at_cursor(
                    cursor, graph, registries, file_imports, file_packages, source, cursor_display,
                );
                let names = child_names(&view, graph);
                assert_eq!(
                    names.len(), *expected_count,
                    "[{}] children_count: expected {}, got {}",
                    cursor_display, expected_count, names.len()
                );
            }
        }
    }
}

// --- Quick-fix helpers ---

/// Execute a single pending quick-fix assertion against the offered
/// `fixes`. Panics (assertion-style) on any unmet expectation; the
/// caller wraps in `catch_unwind` for `expected_failure` handling.
fn run_quick_fix_checks(
    qf: &PendingQuickFix,
    fixes: &[Fix],
    cursor_file: &Path,
    file_sources: &HashMap<PathBuf, String>,
    cursor_display: &str,
) {
    let label = qf.apply_label.as_deref().unwrap_or_else(|| {
        panic!(
            "quick_fix [{}]: call .apply(label) before .run()",
            cursor_display
        )
    });
    let fix = fixes.iter().find(|f| f.label == label).unwrap_or_else(|| {
        panic!(
            "quick_fix [{}]: no fix labeled `{}` was offered; available: {:?}",
            cursor_display,
            label,
            fixes.iter().map(|f| f.label.as_str()).collect::<Vec<_>>()
        )
    });

    let source = file_sources.get(cursor_file).unwrap_or_else(|| {
        panic!(
            "quick_fix [{}]: no source recorded for {}",
            cursor_display,
            cursor_file.display()
        )
    });
    let edited = apply_edits(source, &fix.edits, cursor_file);

    for run in &qf.expected_line_runs {
        assert!(
            contains_trimmed_run(&edited, run),
            "quick_fix [{}]: applied source lacks the expected consecutive \
             lines {:?}\n--- applied source ---\n{}",
            cursor_display,
            run,
            edited
        );
    }
}

/// Apply the subset of `edits` that targets `file` to `source`,
/// bottom-up so earlier offsets stay valid. Columns are interpreted as
/// byte offsets within the line — fixture sources are ASCII, where
/// bytes, chars, and UTF-16 units coincide.
fn apply_edits(source: &str, edits: &[SourceEdit], file: &Path) -> String {
    let mut relevant: Vec<&SourceEdit> = edits
        .iter()
        .filter(|e| e.location.file == file)
        .collect();
    relevant.sort_by_key(|e| (e.location.start_line, e.location.start_col));

    let mut out = source.to_string();
    for edit in relevant.iter().rev() {
        let start = byte_offset(&out, edit.location.start_line, edit.location.start_col);
        let end = byte_offset(&out, edit.location.end_line, edit.location.end_col);
        out.replace_range(start..end, &edit.new_text);
    }
    out
}

/// Byte offset of (zero-based `line`, `col`) in `source`.
fn byte_offset(source: &str, line: u32, col: u32) -> usize {
    let mut offset = 0;
    for _ in 0..line {
        let nl = source[offset..]
            .find('\n')
            .unwrap_or_else(|| panic!("line {} out of bounds for edit target", line));
        offset += nl + 1;
    }
    offset + col as usize
}

/// True iff `expected` occurs in `text` as one consecutive run of
/// lines, with each line compared after trimming.
fn contains_trimmed_run(text: &str, expected: &[String]) -> bool {
    if expected.is_empty() {
        return true;
    }
    let lines: Vec<&str> = text.lines().map(str::trim).collect();
    let needle: Vec<&str> = expected.iter().map(|s| s.trim()).collect();
    if lines.len() < needle.len() {
        return false;
    }
    lines.windows(needle.len()).any(|w| w == needle.as_slice())
}
