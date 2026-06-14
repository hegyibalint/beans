//! The workspace facade — orchestration and workspace-level policy.
//!
//! [`Workspace`] is the product-facing engine surface. It owns a
//! [`Store`] (the raw graph + registries + interner) plus the per-file
//! indexing context that used to live in `beans-lsp::ServerState`:
//! indexed roots, source text, and per-language import/package data. On
//! top of that it exposes the consumer-level API — `update_file`,
//! `remove_file`, `index_workspace`, and the resolution/query methods —
//! so the LSP, a future CLI, or a batch analyzer drive indexing and
//! resolution *through the facade* instead of reimplementing the
//! mechanics (the duplication ADR-0020's library-first rule exists to
//! prevent).
//!
//! Language specifics are gated on their vertical's Cargo feature.
//! Today only Java is wired in; the dispatch in [`Workspace::update_file`]
//! and the parallel scan in [`Workspace::index_workspace`] grow an arm
//! per language as the verticals land. Parsing fans out across rayon
//! (ADR-0005); integration into the graph is serial because the graph
//! and registries are `!Send` (ADR-0018).
//!
//! Both indexing paths converge on one language-neutral serial commit
//! (issue #15): parsing produces an owned, `Send`
//! [`IntegrationJob`](crate::IntegrationJob) per file, and `commit_job`
//! integrates it into the graph on the calling thread — it never names a
//! concrete parsed-file type. Per-language file facts (Java
//! imports/package) live outside the graph because resolution and the
//! unused-import diagnostic consume them directly; a thin per-language
//! wrapper (`commit_java`) records them around the neutral primitive.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use beans_core::graph::NodeId;

use crate::Store;

#[cfg(feature = "java")]
use crate::languages::java::{self, ParsedJavaFile};
#[cfg(feature = "java")]
use crate::view::{DocSymbol, doc_symbol_detail, is_jvm_projection, payload_view};
#[cfg(feature = "java")]
use crate::{Diagnostic, Fix, IntegrationJob, Location, NodePayload};

/// The orchestration facade: a [`Store`] plus the per-file context and
/// the consumer-level API. One per workspace; not `Clone`.
pub struct Workspace {
    store: Store,
    /// Per-file root `NodeId`s. Re-indexing a file destroys these roots
    /// (cascading through hard-link children) before integrating anew;
    /// removing a file destroys them outright.
    file_roots: HashMap<PathBuf, Vec<NodeId>>,
    /// Source text for files the consumer has handed us (open documents).
    /// Bulk indexing does *not* populate this — [`Workspace::source`]
    /// falls back to disk — so a whole-workspace index doesn't pin every
    /// file's text in memory.
    sources: HashMap<PathBuf, String>,
    /// The workspace root, if one was supplied to [`Workspace::index_workspace`].
    root: Option<PathBuf>,
    /// Per-file Java import context, consumed by resolution and diagnostics.
    #[cfg(feature = "java")]
    file_imports: HashMap<PathBuf, Vec<java::Import>>,
    /// Per-file Java package context, consumed by resolution.
    #[cfg(feature = "java")]
    file_packages: HashMap<PathBuf, String>,
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            store: Store::new(),
            file_roots: HashMap::new(),
            sources: HashMap::new(),
            root: None,
            #[cfg(feature = "java")]
            file_imports: HashMap::new(),
            #[cfg(feature = "java")]
            file_packages: HashMap::new(),
        }
    }

    // ---- Storage access ----

    /// The raw storage aggregate. Consumers that need direct graph or
    /// registry access (benchmarks, advanced queries) reach through here;
    /// routine indexing and resolution should use the methods below.
    /// Read-only on purpose: mutating the graph behind the facade's back
    /// would desync the per-file root bookkeeping, so all mutation goes
    /// through `update_file` / `remove_file` / `index_workspace`.
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// The workspace root supplied to [`Workspace::index_workspace`].
    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    /// The source text for `path`: the consumer-supplied buffer if the
    /// file is open, otherwise the on-disk contents. `None` if neither
    /// is available.
    pub fn source(&self, path: &Path) -> Option<String> {
        if let Some(text) = self.sources.get(path) {
            return Some(text.clone());
        }
        std::fs::read_to_string(path).ok()
    }

    // ---- Indexing ----

    /// Index every source file under `root` into the engine. Records
    /// `root` as the workspace root. Returns the number of files that
    /// produced at least one indexed artifact.
    pub fn index_workspace(&mut self, root: &Path) -> usize {
        self.root = Some(root.to_path_buf());
        #[cfg(feature = "java")]
        self.index_java_tree(root);
        self.file_roots.len()
    }

    /// Re-index `path` from the supplied `source`, replacing any prior
    /// roots for the file and caching the text as the open-document
    /// buffer. Dispatches by extension to the owning vertical; a file
    /// whose extension matches no enabled language is cached but
    /// produces no nodes. Returns the inserted `NodeId`s.
    pub fn update_file(&mut self, path: &Path, source: &str) -> Vec<NodeId> {
        self.sources.insert(path.to_path_buf(), source.to_string());
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        match ext {
            #[cfg(feature = "java")]
            "java" => self.commit_java(java::parse_java_to_graph(path, source)),
            _ => {
                // No enabled vertical owns this extension: evict any prior
                // nodes (a file rewritten into an unsupported type must
                // not keep stale symbols) and index nothing further.
                self.destroy_roots(path);
                Vec::new()
            }
        }
    }

    /// Re-index `path` from its on-disk contents. Used when an editor
    /// reports a save without resupplying the text. Returns an empty vec
    /// (and leaves the engine unchanged) if the file can't be read.
    pub fn reindex_from_disk(&mut self, path: &Path) -> Vec<NodeId> {
        match std::fs::read_to_string(path) {
            Ok(source) => self.update_file(path, &source),
            Err(_) => Vec::new(),
        }
    }

    /// Remove `path` from the engine: destroy its roots (cascading
    /// through hard-link children) and drop its cached text and context.
    pub fn remove_file(&mut self, path: &Path) {
        self.destroy_roots(path);
        self.sources.remove(path);
    }

    /// Destroy a file's indexed roots and clear its per-file context,
    /// without touching the cached source. Shared by re-index and remove.
    fn destroy_roots(&mut self, path: &Path) {
        if let Some(old_roots) = self.file_roots.remove(path) {
            for root in old_roots {
                self.store.graph.destroy(root);
            }
        }
        #[cfg(feature = "java")]
        {
            self.file_imports.remove(path);
            self.file_packages.remove(path);
        }
    }

    // ---- Commit path ----

    /// Commit one [`IntegrationJob`] into the engine — the language-neutral
    /// serial integration primitive (ADR-0018), per #15's design. Evicts
    /// the file's prior roots, integrates the job's nodes (the job interns
    /// its own FQNs at this serial boundary — backlog #037), and records
    /// the new roots. Per-language file facts are not its concern; a thin
    /// per-language wrapper (`commit_java`) records those around it. Gated
    /// only because Java is its sole caller today.
    #[cfg(feature = "java")]
    fn commit_job(&mut self, job: Box<dyn IntegrationJob<NodePayload>>) -> Vec<NodeId> {
        let path = job.path().to_path_buf();
        self.destroy_roots(&path);
        let inserted = job.integrate(
            &mut self.store.graph,
            &self.store.registries,
            &self.store.interner,
        );
        self.record_roots(path, &inserted);
        inserted
    }

    /// Record a file's freshly-inserted top-level roots — the ids a later
    /// re-index destroys to refresh the file.
    #[cfg(feature = "java")]
    fn record_roots(&mut self, path: PathBuf, inserted: &[NodeId]) {
        let roots = collect_roots(&self.store, inserted);
        self.file_roots.insert(path, roots);
    }

    // ---- Java vertical ----

    /// Commit a parsed Java file: hand its language-neutral
    /// [`IntegrationJob`] to [`commit_job`](Self::commit_job), then record
    /// the per-file facts the facade keeps outside the graph. Java
    /// imports/package feed resolution and the unused-import diagnostic;
    /// they are moved out of the parse output (which integration ignores)
    /// rather than cloned. `commit_job` already evicted the file's prior
    /// facts via `destroy_roots`, so this only sets the fresh ones.
    #[cfg(feature = "java")]
    fn commit_java(&mut self, mut parsed: ParsedJavaFile) -> Vec<NodeId> {
        let imports = std::mem::take(&mut parsed.imports);
        let package = std::mem::take(&mut parsed.package);
        let path = parsed.path.clone();
        let inserted = self.commit_job(Box::new(parsed));
        self.file_imports.insert(path.clone(), imports);
        if package.is_empty() {
            self.file_packages.remove(&path);
        } else {
            self.file_packages.insert(path, package);
        }
        inserted
    }

    /// Parallel parse + serial commit of every `.java` file under `root`
    /// (ADR-0005). Workers parse into owned, `Send` `ParsedJavaFile`s; the
    /// calling thread drains them through [`commit_java`](Self::commit_java),
    /// the same path incremental updates take. The bulk path is also where
    /// the interner shrinks: a full rescan destroys-then-rebuilds every
    /// root, so we sweep unreferenced interner entries afterward (off the
    /// per-keystroke path, per the `purge` contract — backlog #037).
    #[cfg(feature = "java")]
    fn index_java_tree(&mut self, root: &Path) {
        use rayon::prelude::*;

        let files = scan_java_files(root);

        // `ParsedJavaFile: Send` (static check in beans-lang-java); rayon
        // collects the parsed files on the calling thread.
        let parsed: Vec<ParsedJavaFile> = files
            .par_iter()
            .filter_map(|file| {
                let source = std::fs::read_to_string(file).ok()?;
                Some(java::parse_java_to_graph(file, &source))
            })
            .collect();

        // Serial commit — graph and registries are single-threaded (ADR-0018).
        for parsed in parsed {
            self.commit_java(parsed);
        }

        self.store.interner.purge();
    }

    #[cfg(feature = "java")]
    fn imports_of(&self, path: &Path) -> &[java::Import] {
        self.file_imports
            .get(path)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    #[cfg(feature = "java")]
    fn package_of(&self, path: &Path) -> &str {
        self.file_packages
            .get(path)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    // ---- Resolution / queries ----

    /// Resolve the symbol the cursor sits on to its declaration node.
    /// Tries the dotted name (`Type.member`) first, then the bare
    /// identifier. `None` if nothing resolves.
    ///
    /// Internal: `NodeId` is runtime-only arena identity and must not
    /// cross the public boundary (ADR-0007). Consumers reach resolution
    /// through [`Workspace::definition_at`] / [`Workspace::hover_at`],
    /// which return domain values.
    #[cfg(feature = "java")]
    fn resolve_at(&self, path: &Path, line: u32, col: u32) -> Option<NodeId> {
        let source = self.source(path)?;
        let imports = self.imports_of(path);
        let pkg = self.package_of(path);
        let java = &self.store.registries.java;
        let jvm = &self.store.registries.jvm;

        if let Some(compound) = java::compound_at_position(&source, line, col)
            && let Some(id) =
                java::resolve_compound_name(&compound, imports, pkg, java, jvm, &self.store.graph)
        {
            return Some(id);
        }

        let word = java::word_at_position(&source, line, col)?;
        java::resolve_name(&word, imports, pkg, java, jvm, &self.store.graph)
    }

    /// Resolve the cursor to a declaration and return that declaration's
    /// source location — the go-to-definition primitive.
    #[cfg(feature = "java")]
    pub fn definition_at(&self, path: &Path, line: u32, col: u32) -> Option<Location> {
        let id = self.resolve_at(path, line, col)?;
        let node = self.store.graph.get(id)?;
        payload_view(&node.payload)?.location.cloned()
    }

    /// Resolve the cursor to a declaration and return its payload — the
    /// hover primitive. LSP-shaped formatting (markdown) stays in the
    /// rim (ADR-0020); the facade only locates the payload.
    #[cfg(feature = "java")]
    pub fn hover_at(&self, path: &Path, line: u32, col: u32) -> Option<&crate::NodePayload> {
        let id = self.resolve_at(path, line, col)?;
        self.store.graph.get(id).map(|node| &node.payload)
    }

    /// Find every declaration whose simple name matches the identifier
    /// under the cursor, returning their locations. This mirrors the
    /// prototype's name-based reference search; precise reference
    /// tracking is future work.
    #[cfg(feature = "java")]
    pub fn references_at(&self, path: &Path, line: u32, col: u32) -> Vec<Location> {
        let Some(source) = self.source(path) else {
            return Vec::new();
        };
        let Some(word) = java::word_at_position(&source, line, col) else {
            return Vec::new();
        };
        let mut locations = Vec::new();
        for (_id, node) in self.store.graph.iter() {
            if let Some(view) = payload_view(&node.payload)
                && view.name == word
                && let Some(loc) = view.location
            {
                locations.push(loc.clone());
            }
        }
        locations
    }

    /// The outline (document symbols) for `path`: the file's top-level
    /// declarations and their nested members, as language-neutral
    /// [`DocSymbol`]s. JVM-projection siblings are skipped — only the
    /// source-side declaration becomes a user-facing entry.
    #[cfg(feature = "java")]
    pub fn document_symbols(&self, path: &Path) -> Vec<DocSymbol> {
        let Some(roots) = self.file_roots.get(path) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for &root in roots {
            if let Some(node) = self.store.graph.get(root)
                && is_jvm_projection(&node.payload)
            {
                continue;
            }
            if let Some(sym) = self.build_doc_symbol(path, root) {
                out.push(sym);
            }
        }
        out
    }

    #[cfg(feature = "java")]
    fn build_doc_symbol(&self, file: &Path, id: NodeId) -> Option<DocSymbol> {
        let node = self.store.graph.get(id)?;
        let view = payload_view(&node.payload)?;
        let location = view.location.cloned();

        let children: Vec<DocSymbol> = node
            .children
            .iter()
            .copied()
            .filter_map(|child_id| {
                let child = self.store.graph.get(child_id)?;
                if is_jvm_projection(&child.payload) {
                    return None;
                }
                // A member declared in a *different* file (e.g. an
                // inherited shape) is not part of this file's outline.
                if let Some(child_view) = payload_view(&child.payload)
                    && let Some(loc) = child_view.location
                    && loc.file.as_ref() != file
                {
                    return None;
                }
                self.build_doc_symbol(file, child_id)
            })
            .collect();

        Some(DocSymbol {
            name: view.name.to_string(),
            kind: view.kind,
            detail: doc_symbol_detail(&node.payload),
            location,
            children,
        })
    }

    /// Compute the diagnostics for `path` from the current graph.
    #[cfg(feature = "java")]
    pub fn diagnostics(&self, path: &Path) -> Vec<Diagnostic> {
        let imports = self.imports_of(path);
        let roots = self
            .file_roots
            .get(path)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        crate::compute_diagnostics(
            &self.store.graph,
            &self.store.registries,
            path,
            imports,
            roots,
        )
    }

    /// The quick fixes available at the cursor (e.g. add-import for an
    /// unresolved type). Returns domain [`Fix`]es; the rim maps them onto
    /// protocol code actions.
    #[cfg(feature = "java")]
    pub fn quick_fixes_at(&self, path: &Path, line: u32, col: u32) -> Vec<Fix> {
        let Some(source) = self.source(path) else {
            return Vec::new();
        };
        java::fixes::quick_fixes_at(
            &self.store.graph,
            &self.store.registries.java,
            &self.store.registries.jvm,
            path,
            &source,
            line,
            col,
        )
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}

/// Filter inserted `NodeId`s down to top-level roots (no parent) — the
/// per-file roots re-indexing destroys to refresh the file.
#[cfg(feature = "java")]
fn collect_roots(store: &Store, inserted: &[NodeId]) -> Vec<NodeId> {
    inserted
        .iter()
        .copied()
        .filter(|&id| store.graph.get(id).and_then(|node| node.parent).is_none())
        .collect()
}

/// Scan a directory tree for `.java` files, skipping hidden directories
/// and common build outputs. Cheap directory walk; not parallelised
/// because the OS cost dominates.
#[cfg(feature = "java")]
fn scan_java_files(root: &Path) -> Vec<PathBuf> {
    use walkdir::WalkDir;

    let skip_dirs = [
        "target",
        "build",
        "out",
        "bin",
        ".git",
        ".gradle",
        ".idea",
        "node_modules",
    ];

    WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            if entry.file_type().is_dir() {
                !name.starts_with('.') && !skip_dirs.contains(&name.as_ref())
            } else {
                true
            }
        })
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file() && e.path().extension().is_some_and(|ext| ext == "java")
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

#[cfg(all(test, feature = "java"))]
mod tests {
    use super::*;
    use crate::SymbolKind;

    /// Per ADR-0005 the parse→commit handoff rides a rayon worker, so the
    /// boxed [`IntegrationJob`] must be `Send` (the trait's supertrait
    /// guarantees it). `index_java_tree`'s `collect::<Vec<ParsedJavaFile>>()`
    /// already enforces the parse output is `Send`; this pins the erased
    /// commit job too, and fails loudly if a future change taints it.
    fn _assert_job_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Box<dyn IntegrationJob<NodePayload>>>();
    }

    #[test]
    fn index_workspace_bulk_resolves_across_files() {
        use std::fs;
        let tmp = std::env::temp_dir().join("beans_ws_bulk_index");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("model")).unwrap();
        fs::create_dir_all(tmp.join("service")).unwrap();
        fs::write(
            tmp.join("model/User.java"),
            "package com.example.model;\npublic class User {\n    public String getName() { return null; }\n}\n",
        )
        .unwrap();
        fs::write(
            tmp.join("service/UserService.java"),
            "package com.example.service;\nimport com.example.model.User;\npublic class UserService {\n    public User findUser() { return null; }\n}\n",
        )
        .unwrap();

        let mut ws = Workspace::new();
        let indexed = ws.index_workspace(&tmp);
        assert_eq!(indexed, 2, "both files produced roots");
        assert_eq!(ws.root(), Some(tmp.as_path()));

        // Cross-file resolution works after a *bulk* index — proof the
        // shared commit path recorded each file's imports/package, exactly
        // as the incremental path does.
        let svc = tmp.join("service/UserService.java");
        let user_id = ws
            .resolve_at(&svc, 3, 11)
            .expect("User resolves across files via the import");
        let view = payload_view(&ws.store().graph.get(user_id).unwrap().payload).unwrap();
        assert_eq!(view.fqn, "com.example.model.User");

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn incremental_update_after_bulk_index_refreshes_file() {
        use std::fs;
        let tmp = std::env::temp_dir().join("beans_ws_bulk_then_incremental");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let foo = tmp.join("Foo.java");
        fs::write(
            &foo,
            "package com.test;\npublic class Foo { public void oldMethod() {} }",
        )
        .unwrap();

        let mut ws = Workspace::new();
        ws.index_workspace(&tmp);
        assert!(
            java::lookup_fqn(
                &ws.store().registries.java,
                &ws.store().registries.jvm,
                "com.test.Foo.oldMethod"
            )
            .is_some(),
            "bulk index registers the method"
        );

        // An incremental update over a bulk-indexed file takes the same
        // commit path: the old roots are evicted before the new ones land.
        ws.update_file(
            &foo,
            "package com.test;\npublic class Foo { public void newMethod() {} }",
        );
        assert!(
            java::lookup_fqn(
                &ws.store().registries.java,
                &ws.store().registries.jvm,
                "com.test.Foo.oldMethod"
            )
            .is_none(),
            "old method unregistered after incremental re-index"
        );
        assert!(
            java::lookup_fqn(
                &ws.store().registries.java,
                &ws.store().registries.jvm,
                "com.test.Foo.newMethod"
            )
            .is_some()
        );

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn update_then_resolve_within_file() {
        let mut ws = Workspace::new();
        let path = Path::new("src/Dog.java");
        let source = r#"
package com.example;
public class Dog {
    private String name;
    public String getName() { return name; }
}
"#;
        ws.update_file(path, source);

        // Bare type name resolves to the class declaration.
        let dog_id = ws
            .resolve_at(path, 2, 13)
            .expect("Dog should resolve in com.example");
        let view = payload_view(&ws.store().graph.get(dog_id).unwrap().payload).unwrap();
        assert_eq!(view.fqn, "com.example.Dog");
        assert_eq!(view.kind, SymbolKind::Class);
    }

    #[test]
    fn cross_file_resolution_via_imports() {
        let mut ws = Workspace::new();
        let model = Path::new("src/User.java");
        let svc = Path::new("src/UserService.java");

        ws.update_file(
            model,
            "package com.example.model;\npublic class User {\n    public String getName() { return null; }\n}\n",
        );
        ws.update_file(
            svc,
            "package com.example.service;\nimport com.example.model.User;\npublic class UserService {\n    public User findUser() { return null; }\n}\n",
        );

        // `User` on the return-type line of UserService resolves across
        // files via the explicit import.
        let user_id = ws.resolve_at(svc, 3, 11).expect("import-resolved User");
        let view = payload_view(&ws.store().graph.get(user_id).unwrap().payload).unwrap();
        assert_eq!(view.fqn, "com.example.model.User");
    }

    #[test]
    fn update_file_replaces_old_symbols() {
        let mut ws = Workspace::new();
        let path = Path::new("src/Foo.java");
        ws.update_file(
            path,
            "package com.test;\npublic class Foo { public void oldMethod() {} }",
        );
        assert!(
            java::lookup_fqn(
                &ws.store().registries.java,
                &ws.store().registries.jvm,
                "com.test.Foo.oldMethod"
            )
            .is_some()
        );

        ws.update_file(
            path,
            "package com.test;\npublic class Foo { public void newMethod() {} }",
        );
        assert!(
            java::lookup_fqn(
                &ws.store().registries.java,
                &ws.store().registries.jvm,
                "com.test.Foo.oldMethod"
            )
            .is_none(),
            "old method should be unregistered after re-index"
        );
        assert!(
            java::lookup_fqn(
                &ws.store().registries.java,
                &ws.store().registries.jvm,
                "com.test.Foo.newMethod"
            )
            .is_some()
        );
    }

    #[test]
    fn remove_file_clears_registrations() {
        let mut ws = Workspace::new();
        let path = Path::new("src/Gone.java");
        ws.update_file(path, "package com.test;\npublic class Gone {}");
        assert!(
            java::lookup_fqn(
                &ws.store().registries.java,
                &ws.store().registries.jvm,
                "com.test.Gone"
            )
            .is_some()
        );

        ws.remove_file(path);
        assert!(
            java::lookup_fqn(
                &ws.store().registries.java,
                &ws.store().registries.jvm,
                "com.test.Gone"
            )
            .is_none(),
            "type provider cleared on remove_file"
        );
        assert!(ws.source(path).is_none(), "cached source dropped on remove");
    }

    #[test]
    fn references_finds_every_matching_name() {
        let mut ws = Workspace::new();
        let path = Path::new("/tmp/Service.java");
        let source = "package com.example;\npublic class Service {\n    public void process() {}\n    public void process(String s) {}\n}\n";
        ws.update_file(path, source);

        // Cursor on the first `process` declaration name.
        let locs = ws.references_at(path, 2, 16);
        assert_eq!(locs.len(), 2, "two `process` methods expected");
    }

    #[test]
    fn document_symbols_outline_for_class() {
        let mut ws = Workspace::new();
        let path = Path::new("src/Dog.java");
        ws.update_file(
            path,
            r#"
package com.example;
public class Dog {
    private String name;
    public Dog(String name) { this.name = name; }
    public String getName() { return name; }
}
"#,
        );
        let symbols = ws.document_symbols(path);
        assert_eq!(symbols.len(), 1);
        let dog = &symbols[0];
        assert_eq!(dog.name, "Dog");
        assert_eq!(dog.kind, SymbolKind::Class);

        let names: Vec<&str> = dog.children.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"name"), "name field expected");
        assert!(names.contains(&"Dog"), "constructor expected");
        assert!(names.contains(&"getName"), "getName method expected");

        let getter = dog.children.iter().find(|c| c.name == "getName").unwrap();
        assert_eq!(getter.kind, SymbolKind::Method);
        assert_eq!(getter.detail.as_deref(), Some("() -> String"));
    }

    #[test]
    fn quick_fixes_offer_import_for_unresolved_type() {
        let mut ws = Workspace::new();
        let model = Path::new("/tmp/beans-ws-test/Service.java");
        let app = Path::new("/tmp/beans-ws-test/App.java");
        ws.update_file(
            model,
            "package com.example.model;\npublic class Service {}\n",
        );
        let app_text =
            "package com.example.app;\npublic class App {\n    private Service service;\n}\n";
        ws.update_file(app, app_text);

        // Cursor inside the `Service` use on line 2.
        let fixes = ws.quick_fixes_at(app, 2, 14);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].label, "Import 'com.example.model.Service'");

        // A resolved position offers nothing.
        assert!(ws.quick_fixes_at(app, 1, 0).is_empty());
    }

    #[test]
    fn scan_java_files_skips_hidden_and_build_dirs() {
        use std::fs;
        let tmp = std::env::temp_dir().join("beans_ws_scan");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join(".hidden")).unwrap();
        fs::create_dir_all(tmp.join("target")).unwrap();
        fs::create_dir_all(tmp.join("src")).unwrap();
        fs::write(tmp.join(".hidden/Hidden.java"), "class Hidden {}").unwrap();
        fs::write(tmp.join("target/Built.java"), "class Built {}").unwrap();
        fs::write(tmp.join("src/Visible.java"), "class Visible {}").unwrap();

        let files = scan_java_files(&tmp);
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("Visible.java"));

        let _ = fs::remove_dir_all(&tmp);
    }
}
