use std::collections::HashMap;
use std::path::{Path, PathBuf};

use beans_core::completion::CompletionItems;
use beans_core::language::Language;
use beans_core::resolve::{self, Import};
use beans_core::{Modifier, Signature, Symbol, SymbolKind, SymbolTable};
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
/// Returned by `Fixture::resolve` / `Fixture::resolve("name")`.
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

/// Builder returned after `.complete()` / `.complete("name", ...)`.
/// Allows chaining `.expected_failure()` before `.run()` or further operations.
pub struct CompletionAssert {
    fixture: Fixture,
}

impl CompletionAssert {
    /// Mark the preceding completion assertion as expected to fail.
    pub fn expected_failure(mut self, reason: &str) -> Self {
        if let Some(last) = self.fixture.completions.last_mut() {
            last.mode = TestMode::ExpectedFailure(reason.to_string());
        }
        self
    }

    /// Start resolving at a named cursor. Finalizes and continues.
    pub fn resolve(self, cursor_name: &str) -> CursorAssert {
        self.fixture.resolve(cursor_name)
    }

    /// Start resolving at the anonymous cursor.
    pub fn resolve_default(self) -> CursorAssert {
        self.fixture.resolve_default()
    }

    /// Test completions at another named cursor.
    pub fn complete(self, cursor_name: &str, check: impl FnOnce(&CompletionItems) + Send + 'static) -> CompletionAssert {
        self.fixture.complete(cursor_name, check)
    }

    /// Test completions at the anonymous cursor.
    pub fn complete_default(self, check: impl FnOnce(&CompletionItems) + Send + 'static) -> CompletionAssert {
        self.fixture.complete_default(check)
    }

    // Backward compat
    #[doc(hidden)]
    pub fn assert_at(self, cursor_name: &str) -> CursorAssert {
        self.resolve(cursor_name)
    }

    /// Execute all assertions and completions.
    pub fn run(self) {
        self.fixture.run();
    }
}

// --- Fixture builder ---

/// Test fixture builder. Loads source files, strips cursor markers,
/// builds symbol tables, and runs assertions.
///
/// Language-agnostic: dispatches parsing and word extraction per file extension.
/// No languages are registered by default; add them with `.with_language()`.
pub struct Fixture {
    files: Vec<(PathBuf, String)>,
    assertions: Vec<PendingAssertion>,
    completions: Vec<PendingCompletion>,
    languages: Vec<Box<dyn Language>>,
}

impl Fixture {
    /// Create a new fixture with no languages registered.
    /// Use `.with_language()` to add language support.
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            assertions: Vec::new(),
            completions: Vec::new(),
            languages: Vec::new(),
        }
    }

    /// Register an additional language for this fixture.
    pub fn with_language(mut self, lang: impl Language + 'static) -> Self {
        self.languages.push(Box::new(lang));
        self
    }

    /// Add a source file to the fixture.
    /// The file extension determines which language handles it.
    /// The source may contain `<cur>` / `<cur:name>` markers.
    pub fn file(mut self, path: &str, source: &str) -> Self {
        self.files.push((PathBuf::from(path), source.to_string()));
        self
    }

    /// Resolve the symbol at a named cursor `<cur:name>`.
    pub fn resolve(self, cursor_name: &str) -> CursorAssert {
        CursorAssert {
            fixture: self,
            cursor_name: Some(cursor_name.to_string()),
            checks: Vec::new(),
            mode: TestMode::Normal,
        }
    }

    /// Resolve the symbol at the anonymous `<cur>` cursor.
    pub fn resolve_default(self) -> CursorAssert {
        CursorAssert {
            fixture: self,
            cursor_name: None,
            checks: Vec::new(),
            mode: TestMode::Normal,
        }
    }

    /// Test completions at a named cursor `<cur:name>`.
    pub fn complete(mut self, cursor_name: &str, check: impl FnOnce(&CompletionItems) + Send + 'static) -> CompletionAssert {
        self.completions.push(PendingCompletion {
            cursor_name: Some(cursor_name.to_string()),
            check_fn: Box::new(check),
            mode: TestMode::Normal,
        });
        CompletionAssert { fixture: self }
    }

    /// Test completions at the anonymous `<cur>` cursor.
    pub fn complete_default(mut self, check: impl FnOnce(&CompletionItems) + Send + 'static) -> CompletionAssert {
        self.completions.push(PendingCompletion {
            cursor_name: None,
            check_fn: Box::new(check),
            mode: TestMode::Normal,
        });
        CompletionAssert { fixture: self }
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

    /// Find the registered language for a file extension.
    fn language_for_file(&self, path: &Path) -> &dyn Language {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        for lang in &self.languages {
            if lang.extensions().contains(&ext) {
                return lang.as_ref();
            }
        }
        panic!(
            "no language registered for extension '.{}' (file: {}) — use .with_language()",
            ext,
            path.display()
        );
    }

    /// Execute all pending assertions and completion checks.
    pub fn run(self) {
        // 1. Strip markers from all files
        let mut all_cursors: Vec<CursorPosition> = Vec::new();
        let mut stripped_files: Vec<(PathBuf, String)> = Vec::new();

        for (path, source) in &self.files {
            let stripped = strip_markers(source, path);
            all_cursors.extend(stripped.cursors);
            stripped_files.push((path.clone(), stripped.clean));
        }

        // Validate cursor name uniqueness across files
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

        // 2. Parse all files and build symbol table (dispatching per language)
        let mut table = SymbolTable::new();
        let mut file_imports: HashMap<PathBuf, Vec<Import>> = HashMap::new();
        let mut file_packages: HashMap<PathBuf, String> = HashMap::new();
        let mut file_sources: HashMap<PathBuf, String> = HashMap::new();

        for (path, clean_source) in &stripped_files {
            let lang = self.language_for_file(path);

            let symbols = lang.parse(path, clean_source);
            table.insert_parsed_symbols(symbols);

            let imports = lang.extract_imports(clean_source);
            file_imports.insert(path.clone(), imports);

            let pkg = lang.extract_package(clean_source);
            if !pkg.is_empty() {
                file_packages.insert(path.clone(), pkg);
            }

            file_sources.insert(path.clone(), clean_source.clone());
        }

        // 3. Run resolution assertions
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

            let lang = self.language_for_file(&cursor.file);

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
                            &table,
                            &file_imports,
                            &file_packages,
                            &file_sources,
                            cursor_display,
                            lang,
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
                &table,
                &file_imports,
                &file_packages,
                &file_sources,
                cursor_display,
                lang,
            );
        }

        // 4. Run completion assertions
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

            // Stub: return empty completions for now.
            // As the completion engine is built, this will compute real items.
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
                        Err(_) => {
                            // Expected failure — fine
                        }
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

// --- Assertion execution ---

fn resolve_symbol_at_cursor<'a>(
    cursor: &CursorPosition,
    table: &'a SymbolTable,
    file_imports: &HashMap<PathBuf, Vec<Import>>,
    file_packages: &HashMap<PathBuf, String>,
    source: &str,
    cursor_display: &str,
    lang: &dyn Language,
) -> &'a Symbol {
    let word = lang
        .word_at_position(source, cursor.line, cursor.col)
        .unwrap_or_else(|| {
            panic!(
                "[{}] no word at cursor position ({}:{} in {})",
                cursor_display, cursor.line, cursor.col, cursor.file.display()
            );
        });
    let resolved = resolve::resolve_name(
        &word,
        &cursor.file,
        file_imports,
        file_packages,
        table,
    );
    resolved
        .and_then(|id| table.get(id))
        .unwrap_or_else(|| {
            panic!(
                "[{}] could not resolve '{}' to any symbol",
                cursor_display, word
            );
        })
}

fn run_checks(
    checks: &[AssertionKind],
    cursor: &CursorPosition,
    table: &SymbolTable,
    file_imports: &HashMap<PathBuf, Vec<Import>>,
    file_packages: &HashMap<PathBuf, String>,
    file_sources: &HashMap<PathBuf, String>,
    cursor_display: &str,
    lang: &dyn Language,
) {
    let source = file_sources
        .get(&cursor.file)
        .expect("source not found for cursor file");

    for check in checks {
        match check {
            AssertionKind::ResolvesTo(expected_fqn) => {
                let word = lang
                    .word_at_position(source, cursor.line, cursor.col)
                    .unwrap_or_else(|| {
                        panic!(
                            "[{}] no word at cursor position ({}:{} in {})",
                            cursor_display, cursor.line, cursor.col, cursor.file.display()
                        );
                    });
                let resolved = resolve::resolve_name(
                    &word,
                    &cursor.file,
                    file_imports,
                    file_packages,
                    table,
                );
                let sym = resolved.and_then(|id| table.get(id));
                let sym = sym.unwrap_or_else(|| {
                    panic!(
                        "[{}] could not resolve '{}' to any symbol",
                        cursor_display, word
                    );
                });
                assert_eq!(
                    sym.fqn, *expected_fqn,
                    "[{}] resolves_to: expected '{}', got '{}'",
                    cursor_display, expected_fqn, sym.fqn
                );
            }
            AssertionKind::Kind(expected_kind) => {
                let sym = resolve_symbol_at_cursor(
                    cursor, table, file_imports, file_packages, source, cursor_display, lang,
                );
                assert_eq!(
                    sym.kind, *expected_kind,
                    "[{}] kind: expected {:?}, got {:?}",
                    cursor_display, expected_kind, sym.kind
                );
            }
            AssertionKind::Fqn(expected_fqn) => {
                let sym = resolve_symbol_at_cursor(
                    cursor, table, file_imports, file_packages, source, cursor_display, lang,
                );
                assert_eq!(
                    sym.fqn, *expected_fqn,
                    "[{}] fqn: expected '{}', got '{}'",
                    cursor_display, expected_fqn, sym.fqn
                );
            }
            AssertionKind::Name(expected_name) => {
                let sym = resolve_symbol_at_cursor(
                    cursor, table, file_imports, file_packages, source, cursor_display, lang,
                );
                assert_eq!(
                    sym.name, *expected_name,
                    "[{}] name: expected '{}', got '{}'",
                    cursor_display, expected_name, sym.name
                );
            }
            AssertionKind::HoverContains(text) => {
                let sym = resolve_symbol_at_cursor(
                    cursor, table, file_imports, file_packages, source, cursor_display, lang,
                );
                let hover = resolve::build_hover_text(sym);
                assert!(
                    hover.contains(text.as_str()),
                    "[{}] hover_contains: '{}' not found in hover text:\n{}",
                    cursor_display, text, hover
                );
            }
            AssertionKind::SignatureReturn(expected_ret) => {
                let sym = resolve_symbol_at_cursor(
                    cursor, table, file_imports, file_packages, source, cursor_display, lang,
                );
                match &sym.signature {
                    Some(Signature::Method { return_type, .. }) => {
                        assert_eq!(
                            return_type, expected_ret,
                            "[{}] signature_return: expected '{}', got '{}'",
                            cursor_display, expected_ret, return_type
                        );
                    }
                    other => panic!(
                        "[{}] signature_return: expected Method signature, got {:?}",
                        cursor_display, other
                    ),
                }
            }
            AssertionKind::SignatureParams(expected_params) => {
                let sym = resolve_symbol_at_cursor(
                    cursor, table, file_imports, file_packages, source, cursor_display, lang,
                );
                match &sym.signature {
                    Some(Signature::Method { parameters, .. }) => {
                        assert_eq!(
                            parameters.len(),
                            expected_params.len(),
                            "[{}] signature_params: expected {} params, got {}",
                            cursor_display, expected_params.len(), parameters.len()
                        );
                        for (i, (exp_name, exp_type)) in expected_params.iter().enumerate() {
                            assert_eq!(
                                parameters[i].name, *exp_name,
                                "[{}] param[{}] name: expected '{}', got '{}'",
                                cursor_display, i, exp_name, parameters[i].name
                            );
                            assert_eq!(
                                parameters[i].param_type, *exp_type,
                                "[{}] param[{}] type: expected '{}', got '{}'",
                                cursor_display, i, exp_type, parameters[i].param_type
                            );
                        }
                    }
                    other => panic!(
                        "[{}] signature_params: expected Method signature, got {:?}",
                        cursor_display, other
                    ),
                }
            }
            AssertionKind::Modifiers(expected_mods) => {
                let sym = resolve_symbol_at_cursor(
                    cursor, table, file_imports, file_packages, source, cursor_display, lang,
                );
                for m in expected_mods {
                    assert!(
                        sym.modifiers.contains(m),
                        "[{}] modifiers: expected {:?} but symbol has {:?}",
                        cursor_display, m, sym.modifiers
                    );
                }
            }
            AssertionKind::ParentFqn(expected_fqn) => {
                let sym = resolve_symbol_at_cursor(
                    cursor, table, file_imports, file_packages, source, cursor_display, lang,
                );
                let parent = sym.parent.and_then(|pid| table.get(pid));
                let parent = parent.unwrap_or_else(|| {
                    panic!("[{}] parent_fqn: symbol has no parent", cursor_display);
                });
                assert_eq!(
                    parent.fqn, *expected_fqn,
                    "[{}] parent_fqn: expected '{}', got '{}'",
                    cursor_display, expected_fqn, parent.fqn
                );
            }
            AssertionKind::ChildrenInclude(expected_names) => {
                let sym = resolve_symbol_at_cursor(
                    cursor, table, file_imports, file_packages, source, cursor_display, lang,
                );
                let child_names: Vec<String> = table
                    .lookup_children(sym.id)
                    .into_iter()
                    .filter_map(|cid| table.get(cid))
                    .map(|c| c.name.clone())
                    .collect();
                for name in expected_names {
                    assert!(
                        child_names.contains(name),
                        "[{}] children_include: '{}' not found among children {:?}",
                        cursor_display, name, child_names
                    );
                }
            }
            AssertionKind::ChildrenCount(expected_count) => {
                let sym = resolve_symbol_at_cursor(
                    cursor, table, file_imports, file_packages, source, cursor_display, lang,
                );
                let count = table.lookup_children(sym.id).len();
                assert_eq!(
                    count, *expected_count,
                    "[{}] children_count: expected {}, got {}",
                    cursor_display, expected_count, count
                );
            }
        }
    }
}
