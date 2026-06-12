//! Candidate normalization: raw supplier output → unique, validated
//! Java homes.
//!
//! Mirrors Gradle's `DefaultJavaInstallationRegistry.collectInstallations`
//! rules: canonicalize (symlinks), unwrap macOS `Contents/Home`, unwrap
//! one level of archive nesting (the `~/.gradle/jdks` shape, where the
//! supplier-visible directory contains the actual extracted JDK),
//! require `bin/java`, and dedupe by canonical path while merging the
//! source tags.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::suppliers::Candidate;
use crate::JavaInstallation;

fn has_java_executable(home: &Path) -> bool {
    let bin = home.join("bin");
    bin.join("java").is_file() || bin.join("java.exe").is_file()
}

/// Resolve a raw candidate path to a Java home, or `None` if nothing
/// java-shaped is there.
fn resolve_home(raw: &Path) -> Option<PathBuf> {
    let path = raw.canonicalize().ok()?;
    if !path.is_dir() {
        return None;
    }

    if has_java_executable(&path) {
        return Some(path);
    }

    // macOS bundle: <dir>/Contents/Home.
    let mac_home = path.join("Contents/Home");
    if has_java_executable(&mac_home) {
        return Some(mac_home);
    }

    // One level of archive nesting: <dir>/<extracted-jdk>/(bin|Contents/Home).
    // Take the unwrap only when it is unambiguous.
    let mut nested: Vec<PathBuf> = Vec::new();
    for entry in std::fs::read_dir(&path).ok()?.flatten() {
        let child = entry.path();
        if !child.is_dir() {
            continue;
        }
        if has_java_executable(&child) {
            nested.push(child);
        } else {
            let mac = child.join("Contents/Home");
            if has_java_executable(&mac) {
                nested.push(mac);
            }
        }
    }
    match nested.as_slice() {
        [single] => Some(single.clone()),
        _ => None,
    }
}

/// Normalize every candidate and dedupe by canonical home, merging the
/// source tags of duplicates. Order is deterministic (sorted by path).
pub fn normalize_and_dedup(raw: &[Candidate]) -> Vec<JavaInstallation> {
    let mut by_home: HashMap<PathBuf, Vec<String>> = HashMap::new();
    for cand in raw {
        let Some(home) = resolve_home(&cand.path) else {
            continue;
        };
        let sources = by_home.entry(home).or_default();
        if !sources.contains(&cand.source) {
            sources.push(cand.source.clone());
        }
    }
    let mut out: Vec<JavaInstallation> = by_home
        .into_iter()
        .map(|(java_home, sources)| JavaInstallation {
            java_home,
            sources,
            metadata: None,
        })
        .collect();
    out.sort_by(|a, b| a.java_home.cmp(&b.java_home));
    out
}
