//! Metadata probing: release-file parse first, bounded exec fallback.
//!
//! Every JDK 9+ (and most vendor 8s) ships `<home>/release` with
//! `KEY="value"` lines — `JAVA_VERSION`, `IMPLEMENTOR`, `OS_ARCH`.
//! Parsing it costs microseconds and no process. The exec fallback
//! (`java -XshowSettings:properties -version`, properties on stderr)
//! exists for homes without a usable release file and runs only when
//! selection actually considers such a candidate; it is killed after a
//! timeout so a broken installation cannot hang the caller.

use std::io::Read;
use std::path::Path;
use std::time::Duration;

/// Probed facts about an installation. `major` is the comparison key;
/// the rest is for display and (later) vendor matching.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JvmMetadata {
    /// Feature release: 8, 11, 17, 21, ...
    pub major: u32,
    /// Full version string, e.g. `21.0.2` or `1.8.0_392`.
    pub version: String,
    /// `IMPLEMENTOR` / `java.vendor`, when stated.
    pub vendor: Option<String>,
    /// `OS_ARCH` / `os.arch`, when stated.
    pub arch: Option<String>,
}

/// `"21.0.2"` → 21; `"1.8.0_392"` → 8 (pre-9 `1.x` scheme).
fn major_of(version: &str) -> Option<u32> {
    let mut parts = version.split(['.', '_', '+', '-']);
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

/// Parse one `KEY="value"` line from a release file.
fn release_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(key)?.strip_prefix('=')?;
    Some(rest.trim().trim_matches('"'))
}

/// Probe via `<home>/release`. `None` when the file is absent or
/// carries no parseable `JAVA_VERSION`.
pub fn from_release_file(home: &Path) -> Option<JvmMetadata> {
    let content = std::fs::read_to_string(home.join("release")).ok()?;
    let mut version = None;
    let mut vendor = None;
    let mut arch = None;
    for line in content.lines() {
        if let Some(v) = release_value(line, "JAVA_VERSION") {
            version = Some(v.to_string());
        } else if let Some(v) = release_value(line, "IMPLEMENTOR") {
            vendor = Some(v.to_string());
        } else if let Some(v) = release_value(line, "OS_ARCH") {
            arch = Some(v.to_string());
        }
    }
    let version = version?;
    Some(JvmMetadata {
        major: major_of(&version)?,
        version,
        vendor,
        arch,
    })
}

/// Exec-probe timeout. Generous for a JVM that prints settings and
/// exits; tight enough that a wedged installation cannot stall
/// detection noticeably.
const EXEC_TIMEOUT: Duration = Duration::from_secs(10);

/// Probe by running `bin/java -XshowSettings:properties -version` and
/// parsing the property dump from stderr. Bounded by [`EXEC_TIMEOUT`];
/// the child is killed on expiry.
pub fn from_exec(home: &Path) -> Option<JvmMetadata> {
    let exe = if cfg!(windows) { "java.exe" } else { "java" };
    let mut child = std::process::Command::new(home.join("bin").join(exe))
        .args(["-XshowSettings:properties", "-version"])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;

    let mut stderr = child.stderr.take()?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = stderr.read_to_string(&mut buf);
        let _ = tx.send(buf);
    });

    let output = match rx.recv_timeout(EXEC_TIMEOUT) {
        Ok(buf) => buf,
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
    };
    let _ = child.wait();

    let prop = |name: &str| {
        output.lines().find_map(|l| {
            let l = l.trim();
            l.strip_prefix(name)
                .and_then(|r| r.trim_start().strip_prefix('='))
                .map(|v| v.trim().to_string())
        })
    };

    let version = prop("java.version")?;
    Some(JvmMetadata {
        major: major_of(&version)?,
        version,
        vendor: prop("java.vendor"),
        arch: prop("os.arch"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn major_parsing_handles_both_version_schemes() {
        assert_eq!(major_of("21.0.2"), Some(21));
        assert_eq!(major_of("17"), Some(17));
        assert_eq!(major_of("1.8.0_392"), Some(8));
        assert_eq!(major_of("11.0.22+7"), Some(11));
        assert_eq!(major_of("nonsense"), None);
    }
}
