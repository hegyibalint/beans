//! Workspace indexing for the LSP.
//!
//! Per ADR-0005 ("sync core, rayon parallelism at the file batch")
//! the parse phase fans out across rayon workers and the integrate
//! phase runs serially on the graph thread. `parse_java_to_graph`
//! produces a self-contained `ParsedJavaFile` (verified `Send` per
//! the static check in `beans-core`); `integrate` consumes the plan
//! against the live graph + registries.

use std::path::{Path, PathBuf};

use beans::graph::{Graph, NodeId};
use beans::languages::java::{self, ParsedJavaFile};
use beans::payload::NodePayload;
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::backend::ServerState;

/// Scan the workspace for `.java` files, skipping hidden and build
/// directories. Cheap directory walk; not parallelised because the OS
/// cost dominates.
pub fn scan_workspace(root: &Path) -> Vec<PathBuf> {
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

/// Parse a single source string and integrate the result into the
/// graph. Used by both the workspace-startup pass and the per-file
/// `did_change` re-index path.
pub fn integrate_source(state: &mut ServerState, file: &Path, source: &str) -> Vec<NodeId> {
    // Destroy any previously-indexed roots for this file so the
    // refreshed graph stays consistent with the new source.
    if let Some(old_roots) = state.file_roots.remove(file) {
        for root in old_roots {
            state.beans.graph.destroy(root);
        }
    }

    let parsed = java::parse_java_to_graph(file, source);
    state
        .file_imports
        .insert(file.to_path_buf(), parsed.imports.clone());
    if !parsed.package.is_empty() {
        state
            .file_packages
            .insert(file.to_path_buf(), parsed.package.clone());
    } else {
        state.file_packages.remove(file);
    }

    // `integrate` interns FQNs at the serial boundary (backlog #037).
    let inserted = java::integrate(
        &mut state.beans.graph,
        &state.beans.registries,
        &state.beans.interner,
        parsed,
    );
    let roots = collect_roots(&state.beans.graph, &inserted);
    state.file_roots.insert(file.to_path_buf(), roots);
    inserted
}

/// Index every Java file under `root` into `state`. Parsing runs in
/// parallel on the rayon pool; integrate runs serially on the calling
/// thread (the graph and its registries are `!Send` per ADR-0018).
pub fn index_workspace(root: &Path, state: &mut ServerState) {
    let files = scan_workspace(root);

    // Parallel parse phase. `ParsedJavaFile: Send` per the static
    // check in `beans-core/src/languages/java/parser.rs`; rayon
    // collects the outputs into a Vec on the calling thread.
    let parsed: Vec<(PathBuf, ParsedJavaFile)> = files
        .par_iter()
        .filter_map(|file| {
            let source = std::fs::read_to_string(file).ok()?;
            Some((file.clone(), java::parse_java_to_graph(file, &source)))
        })
        .collect();

    // Serial integrate phase: registries' interior mutability requires
    // single-thread access (per ADR-0018), and parents must be inserted
    // before children. The plan order inside each `ParsedJavaFile` is
    // already topological; cross-file ordering doesn't matter for
    // hard-link parents (those are intra-file).
    for (path, plan) in parsed {
        if let Some(old_roots) = state.file_roots.remove(&path) {
            for root in old_roots {
                state.beans.graph.destroy(root);
            }
        }
        state
            .file_imports
            .insert(path.clone(), plan.imports.clone());
        if !plan.package.is_empty() {
            state
                .file_packages
                .insert(path.clone(), plan.package.clone());
        }
        // `integrate` interns FQNs at the serial boundary (backlog #037).
        let inserted = java::integrate(
            &mut state.beans.graph,
            &state.beans.registries,
            &state.beans.interner,
            plan,
        );
        let roots = collect_roots(&state.beans.graph, &inserted);
        state.file_roots.insert(path, roots);
    }

    // Bulk reindex is where the name population actually shrinks (a full
    // rescan destroys-then-rebuilds every root). Sweep interner entries
    // no surviving node references — off the per-keystroke path, per the
    // `purge` contract (backlog #037).
    state.beans.interner.purge();
}

/// Filter the inserted NodeIds down to those that are top-level
/// (no parent) — the per-file roots the LSP destroys to refresh.
fn collect_roots(graph: &Graph<NodePayload>, inserted: &[NodeId]) -> Vec<NodeId> {
    inserted
        .iter()
        .copied()
        .filter(|&id| graph.get(id).and_then(|node| node.parent).is_none())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_workspace_skips_hidden_and_build_dirs() {
        use std::fs;
        let tmp = std::env::temp_dir().join("beans_test_scan");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join(".hidden")).unwrap();
        fs::create_dir_all(tmp.join("target")).unwrap();
        fs::create_dir_all(tmp.join("src")).unwrap();
        fs::write(tmp.join(".hidden/Hidden.java"), "class Hidden {}").unwrap();
        fs::write(tmp.join("target/Built.java"), "class Built {}").unwrap();
        fs::write(tmp.join("src/Visible.java"), "class Visible {}").unwrap();

        let files = scan_workspace(&tmp);
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("Visible.java"));

        let _ = fs::remove_dir_all(&tmp);
    }
}
