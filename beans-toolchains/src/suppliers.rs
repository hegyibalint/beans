//! Detection sources: where JAVA_HOME-compatible directories live.
//!
//! Each supplier is a plain function from explicit roots to candidate
//! paths, so tests can point them at fixture trees;
//! [`real_candidates`] assembles them against the actual host
//! environment. Suppliers over-report freely — normalization
//! (existence, `bin/java`, canonicalization, dedup) happens downstream
//! in [`crate::candidate`].

use std::path::{Path, PathBuf};

/// A raw candidate: a path some supplier believes might be a Java
/// home, tagged with the supplier's name for diagnostics.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub path: PathBuf,
    pub source: String,
}

impl Candidate {
    fn new(path: PathBuf, source: &str) -> Self {
        Self {
            path,
            source: source.to_string(),
        }
    }
}

/// `$JAVA_HOME`, verbatim.
pub fn java_home_env(value: Option<&str>) -> Vec<Candidate> {
    value
        .filter(|v| !v.is_empty())
        .map(|v| vec![Candidate::new(PathBuf::from(v), "JAVA_HOME")])
        .unwrap_or_default()
}

/// The `java` on PATH, resolved through symlinks back to its home
/// (`<home>/bin/java` → `<home>`).
pub fn path_java(path_var: Option<&str>) -> Vec<Candidate> {
    let Some(path_var) = path_var else {
        return Vec::new();
    };
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    for dir in std::env::split_paths(path_var) {
        let java = dir.join(exe);
        if !java.is_file() {
            continue;
        }
        // Resolve symlink chains (e.g. /usr/bin/java -> .../zulu/bin/java)
        // before walking up to the home.
        let resolved = java.canonicalize().unwrap_or(java);
        if let Some(home) = resolved.parent().and_then(Path::parent) {
            return vec![Candidate::new(home.to_path_buf(), "PATH")];
        }
    }
    Vec::new()
}

/// Scan the immediate children of `dir` as candidates — the shape of
/// every version-manager and tool-cache directory.
pub fn dir_children(dir: &Path, source: &str) -> Vec<Candidate> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .map(|p| Candidate::new(p, source))
        .collect()
}

/// Maven's `~/.m2/toolchains.xml`: `<jdkHome>` entries, with
/// `${env.NAME}` substitution. Parsed with a string scan — the file is
/// tiny and flat; an XML dependency is not worth the one element we
/// read. Over-reporting on a malformed file is harmless (downstream
/// validation drops nonsense).
pub fn maven_toolchains(xml: &str, env: &dyn Fn(&str) -> Option<String>) -> Vec<Candidate> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<jdkHome>") {
        rest = &rest[start + "<jdkHome>".len()..];
        let Some(end) = rest.find("</jdkHome>") else {
            break;
        };
        let raw = rest[..end].trim();
        rest = &rest[end..];

        let mut value = String::new();
        let mut remaining = raw;
        while let Some(open) = remaining.find("${env.") {
            value.push_str(&remaining[..open]);
            let after = &remaining[open + "${env.".len()..];
            let Some(close) = after.find('}') else {
                remaining = "";
                break;
            };
            if let Some(v) = env(&after[..close]) {
                value.push_str(&v);
            }
            remaining = &after[close + 1..];
        }
        value.push_str(remaining);
        if !value.is_empty() {
            out.push(Candidate::new(PathBuf::from(value), "Maven toolchains.xml"));
        }
    }
    out
}

/// All suppliers against the real host environment.
pub fn real_candidates() -> Vec<Candidate> {
    let env = |k: &str| std::env::var(k).ok();
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);

    let mut out = Vec::new();
    out.extend(java_home_env(env("JAVA_HOME").as_deref()));
    out.extend(path_java(env("PATH").as_deref()));

    if let Some(home) = &home {
        // Version managers.
        let sdkman = env("SDKMAN_CANDIDATES_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".sdkman/candidates"));
        out.extend(dir_children(&sdkman.join("java"), "SDKMAN"));

        let asdf = env("ASDF_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".asdf"));
        out.extend(dir_children(&asdf.join("installs/java"), "asdf"));

        let mise = env("MISE_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share/mise"));
        out.extend(dir_children(&mise.join("installs/java"), "mise"));

        // Tool caches.
        out.extend(dir_children(&home.join(".jdks"), "IntelliJ ~/.jdks"));
        let gradle_home = env("GRADLE_USER_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".gradle"));
        out.extend(dir_children(
            &gradle_home.join("jdks"),
            "Gradle ~/.gradle/jdks",
        ));

        let toolchains_xml = home.join(".m2/toolchains.xml");
        if let Ok(xml) = std::fs::read_to_string(&toolchains_xml) {
            out.extend(maven_toolchains(&xml, &env));
        }
    }

    // OS conventions.
    #[cfg(target_os = "macos")]
    {
        out.extend(dir_children(
            Path::new("/Library/Java/JavaVirtualMachines"),
            "macOS JavaVirtualMachines",
        ));
        if let Some(home) = &home {
            out.extend(dir_children(
                &home.join("Library/Java/JavaVirtualMachines"),
                "macOS user JavaVirtualMachines",
            ));
        }
    }
    #[cfg(target_os = "linux")]
    {
        for dir in [
            "/usr/lib/jvm",
            "/usr/lib64/jvm",
            "/usr/java",
            "/usr/local/java",
            "/opt/java",
        ] {
            out.extend(dir_children(Path::new(dir), "Linux convention"));
        }
    }
    #[cfg(target_os = "windows")]
    {
        for vendor_dir in [
            "Java",
            "Eclipse Adoptium",
            "Eclipse Foundation",
            "Amazon Corretto",
            "Microsoft",
            "Zulu",
            "BellSoft",
            "Semeru",
        ] {
            for pf in ["C:\\Program Files", "C:\\Program Files (x86)"] {
                out.extend(dir_children(
                    &Path::new(pf).join(vendor_dir),
                    "Windows Program Files",
                ));
            }
        }
    }

    out
}
