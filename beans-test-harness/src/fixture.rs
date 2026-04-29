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

use beans_core::completion::CompletionItems;
use beans_core::graph::{Graph, NodeId};
use beans_core::resolve::Import;
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
    check_fn: Box<dyn FnOnce(&CompletionItems) + Send>,
    mode: TestMode,
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

    pub fn complete(self, cursor_name: &str, check: impl FnOnce(&CompletionItems) + Send + 'static) -> CompletionAssert {
        self.fixture.complete(cursor_name, check)
    }

    pub fn complete_default(self, check: impl FnOnce(&CompletionItems) + Send + 'static) -> CompletionAssert {
        self.fixture.complete_default(check)
    }

    #[doc(hidden)]
    pub fn assert_at(self, cursor_name: &str) -> CursorAssert {
        self.resolve(cursor_name)
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

    pub fn complete(mut self, cursor_name: &str, check: impl FnOnce(&CompletionItems) + Send + 'static) -> CompletionAssert {
        self.completions.push(PendingCompletion {
            cursor_name: Some(cursor_name.to_string()),
            check_fn: Box::new(check),
            mode: TestMode::Normal,
        });
        CompletionAssert { fixture: self }
    }

    pub fn complete_default(mut self, check: impl FnOnce(&CompletionItems) + Send + 'static) -> CompletionAssert {
        self.completions.push(PendingCompletion {
            cursor_name: None,
            check_fn: Box::new(check),
            mode: TestMode::Normal,
        });
        CompletionAssert { fixture: self }
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

            let items = CompletionItems(Vec::new());

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

fn parse_for_extension(path: &Path, source: &str) -> ParsedForFixture {
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
        NodePayload::Java(java_payload) => Some(java_view_fields(java_payload)),
        NodePayload::Jvm(jvm_payload) => Some(jvm_view_fields(jvm_payload)),
    }
}

#[cfg(feature = "java")]
fn java_view_fields(
    payload: &beans_core::languages::java::JavaNodePayload,
) -> (String, String, SymbolKind, Vec<Modifier>) {
    use beans_core::languages::java::{JavaNodePayload, JavaTypeKind};
    match payload {
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
    }
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
    let id = java::resolve_name(&word, imports, current_package, registries, graph)
        .unwrap_or_else(|| {
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

    let kind_str = match view.kind {
        SymbolKind::Class => "class",
        SymbolKind::Interface => "interface",
        SymbolKind::Enum => "enum",
        SymbolKind::Record => "record",
        SymbolKind::Annotation => "@interface",
        SymbolKind::Method => "method",
        SymbolKind::Constructor => "constructor",
        SymbolKind::Field => "field",
        SymbolKind::EnumConstant => "field",
        SymbolKind::Parameter => "parameter",
        SymbolKind::Package => "package",
        _ => "symbol",
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
            if let Some(header) = match &child.payload {
                #[cfg(feature = "java")]
                NodePayload::Java(j) => j.header(),
                NodePayload::Jvm(_) => None,
            } {
                out.push(header.name.clone());
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
