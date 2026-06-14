//! Index a sidecar-imported workspace through the [`beans::Workspace`]
//! facade and report what that costs — the real-world measurement
//! harness, and a worked example of a non-LSP consumer driving the
//! facade directly.
//!
//! ```text
//! cargo run --release -p beans --example index_workspace -- \
//!     /tmp/gradle-master-import.json [--tests]
//! ```
//!
//! Reads a `WorkspaceModel` JSON (the sidecar's `gradle/import` result),
//! indexes each source root through [`beans::Workspace::index_workspace`]
//! — the production path: parallel parse, serial integrate, interner
//! purge (ADR-0005) — then times a few engine queries at scale and
//! exercises the facade's diagnostics/outline API. Bulk indexing does
//! not retain source text, so the memory figures reflect the graph, not
//! buffered source.

use std::path::{Path, PathBuf};
use std::time::Instant;

use beans::languages::java;
use beans::{Store, Workspace};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let model_path = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .cloned()
        .unwrap_or_else(|| "/tmp/gradle-master-import.json".to_string());
    let include_tests = args.iter().any(|a| a == "--tests");

    let model: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&model_path).expect("read model"))
            .expect("parse model");

    let mut roots: Vec<PathBuf> = Vec::new();
    for module in model["modules"].as_array().expect("modules") {
        for key in ["sourceRoots", "generatedSourceRoots"] {
            collect_roots(&module[key], &mut roots);
        }
        if include_tests {
            collect_roots(&module["testSourceRoots"], &mut roots);
        }
    }
    roots.sort();
    roots.dedup();

    // Discover files up front for the count + byte total (metadata only,
    // no reads — the facade does the reading during indexing).
    let mut files: Vec<PathBuf> = Vec::new();
    for root in &roots {
        walk_java(root, &mut files);
    }
    let bytes: u64 = files
        .iter()
        .filter_map(|f| std::fs::metadata(f).ok())
        .map(|m| m.len())
        .sum();
    println!(
        "{} roots ({}tests), {} .java files, {:.1} MB",
        roots.len(),
        if include_tests { "with " } else { "no " },
        files.len(),
        bytes as f64 / 1e6
    );

    // Index every source root through the facade. This is the same path
    // the LSP drives on startup: parallel parse, serial integrate, and a
    // post-pass interner purge.
    let mut ws = Workspace::new();
    let t_index = Instant::now();
    let mut indexed = 0;
    for root in &roots {
        indexed = ws.index_workspace(root);
    }
    let index_elapsed = t_index.elapsed();

    let nodes = ws.store().graph.iter().count();
    memory_anatomy(ws.store());
    println!(
        "index: {index_elapsed:.2?}  ({indexed} files indexed, {nodes} graph nodes, \
         {:.0} files/s, {:.0} nodes/ms)",
        indexed as f64 / index_elapsed.as_secs_f64().max(1e-6),
        nodes as f64 / index_elapsed.as_millis().max(1) as f64
    );
    println!("rss:       {} MB", rss_mb());

    // Engine queries at scale, through the facade's storage handle.
    let store = ws.store();
    let t = Instant::now();
    let hits = store.registries.java.symbols.query_simple_name("Project");
    println!(
        "query_simple_name(\"Project\"): {} keys in {:.2?}",
        hits.len(),
        t.elapsed()
    );

    let t = Instant::now();
    let resolved = java::lookup_fqn(
        &store.registries.java,
        &store.registries.jvm,
        "org.gradle.api.Project",
    );
    println!(
        "lookup_fqn(org.gradle.api.Project): {:?} in {:.2?}",
        resolved.is_some(),
        t.elapsed()
    );

    // Facade query API: diagnostics + outline for the first indexed file.
    if let Some(file) = files.first() {
        let name = file.file_name().unwrap().to_string_lossy();
        let t = Instant::now();
        let diags = ws.diagnostics(file);
        println!(
            "diagnostics({name}): {} findings in {:.2?}",
            diags.len(),
            t.elapsed()
        );
        let symbols = ws.document_symbols(file);
        println!(
            "document_symbols({name}): {} top-level symbols",
            symbols.len()
        );
    }
}

/// Approximate per-category heap anatomy of the graph — the share
/// measurement gating backlog #037's strong-form design.
fn memory_anatomy(store: &Store) {
    use beans::NodePayload;
    use beans::jvm::{JvmNodePayload, TypeRef};
    use beans::languages::java::JavaNodePayload;
    use std::collections::HashSet;

    fn typeref_stats(t: &TypeRef, count: &mut u64, text: &mut u64) {
        *count += 1;
        match t {
            TypeRef::Simple { name } => *text += name.capacity() as u64,
            TypeRef::TypeVariable { name } => *text += name.capacity() as u64,
            TypeRef::Parameterized { raw, args } => {
                typeref_stats(raw, count, text);
                args.iter().for_each(|a| typeref_stats(a, count, text));
            }
            TypeRef::Array { element } => typeref_stats(element, count, text),
            TypeRef::Wildcard { .. } | TypeRef::Intersection { .. } => {}
            _ => {}
        }
    }

    let mut fqn_refs: u64 = 0;
    let mut fqn_buffers: HashSet<*const u8> = HashSet::new();
    let mut fqn_buffer_bytes: u64 = 0;
    let mut name_bytes: u64 = 0;
    let mut candidate_refs: u64 = 0;
    let mut tr_count: u64 = 0;
    let mut tr_text: u64 = 0;
    let mut handle_count: u64 = 0;
    let mut locations: u64 = 0;

    let mut see_fqn = |f: &beans::Fqn| {
        fqn_refs += 1;
        let ptr = f.as_str().as_ptr();
        if fqn_buffers.insert(ptr) {
            fqn_buffer_bytes += f.as_str().len() as u64;
        }
    };

    for (_id, node) in store.graph.iter() {
        handle_count += node.handles.len() as u64;
        match &node.payload {
            NodePayload::Java(j) => {
                if let Some(h) = j.header() {
                    see_fqn(&h.fqn);
                    name_bytes += h.name.capacity() as u64;
                    if h.location.is_some() {
                        locations += 1;
                    }
                }
                match j {
                    JavaNodePayload::TypeUse(t) => {
                        locations += 1;
                        for f in &t.header.candidate_fqns {
                            candidate_refs += 1;
                            see_fqn(f);
                        }
                        name_bytes += t.header.name.capacity() as u64;
                    }
                    JavaNodePayload::Method(m) => {
                        typeref_stats(&m.return_type, &mut tr_count, &mut tr_text);
                        for p in &m.parameters {
                            typeref_stats(&p.param_type, &mut tr_count, &mut tr_text);
                            name_bytes += p.name.capacity() as u64;
                        }
                    }
                    JavaNodePayload::Field(f) => {
                        typeref_stats(&f.field_type, &mut tr_count, &mut tr_text);
                    }
                    _ => {}
                }
            }
            NodePayload::Jvm(v) => {
                if let Some(h) = v.header() {
                    see_fqn(&h.fqn);
                    name_bytes += h.name.capacity() as u64;
                }
                if let JvmNodePayload::Method(m) = v {
                    see_fqn(&m.owner);
                    typeref_stats(&m.return_type, &mut tr_count, &mut tr_text);
                    for p in &m.parameters {
                        typeref_stats(&p.param_type, &mut tr_count, &mut tr_text);
                        name_bytes += p.name.capacity() as u64;
                    }
                }
            }
        }
    }

    let slots = store.graph.iter().count() as u64;
    let payload_width = std::mem::size_of::<NodePayload>() as u64;
    println!(
        "
-- memory anatomy (approx, heap-categories) --"
    );
    println!(
        "  payload enum width:    {} B x {} slots = {:.0} MB (arena floor)",
        payload_width,
        slots,
        (payload_width * slots) as f64 / 1e6
    );
    println!(
        "  fqn references:        {} ({} distinct buffers, {:.1} MB text)",
        fqn_refs,
        fqn_buffers.len(),
        fqn_buffer_bytes as f64 / 1e6
    );
    println!("    of which candidates: {}", candidate_refs);
    println!("  name field text:       {:.1} MB", name_bytes as f64 / 1e6);
    println!(
        "  TypeRef nodes:         {} ({:.1} MB text, ~{:.0} MB structs)",
        tr_count,
        tr_text as f64 / 1e6,
        (tr_count * std::mem::size_of::<TypeRef>() as u64) as f64 / 1e6
    );
    println!(
        "  RAII handles:          {} (~{:.0} MB boxed)",
        handle_count,
        (handle_count * 88) as f64 / 1e6
    );
    println!("  locations:             {}", locations);
}

fn collect_roots(value: &serde_json::Value, out: &mut Vec<PathBuf>) {
    if let Some(arr) = value.as_array() {
        out.extend(arr.iter().filter_map(|v| v.as_str()).map(PathBuf::from));
    }
}

fn walk_java(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_java(&path, out);
        } else if path.extension().is_some_and(|e| e == "java") {
            out.push(path);
        }
    }
}

fn rss_mb() -> u64 {
    std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|kb| kb / 1024)
        .unwrap_or(0)
}
