//! Test-only JDK provisioning: download, cache, and serve real JDKs.
//!
//! Tests that read real JDK artifacts (jmods, jars) call
//! [`jdk`] with a feature version and get a usable JDK home back,
//! independent of whatever `$JAVA_HOME` happens to point at (which may
//! be a runtime without `jmods/`, like a JetBrains "nomod" build).
//!
//! On first use a pinned-major Eclipse Temurin build is downloaded from
//! the Adoptium API and cached under `~/.cache/beans/test-jdks`
//! (override with `BEANS_TEST_JDK_CACHE`). Each run checks the latest
//! patch release with a HEAD request (the Adoptium `binary/latest` URL
//! redirects to an asset path carrying the release tag) and fetches it
//! when the cache is behind; older patches are pruned. When the check
//! fails — offline — the newest cached build serves instead, so the
//! network is only ever *required* on a cold cache.
//!
//! Downloads shell out to `curl` and `tar` — dev-machine tools — so
//! this crate carries no HTTP/TLS dependencies into the workspace.
//!
//! Cross-process races are handled by extracting into a temp directory
//! and renaming into the cache slot; the loser discards its copy.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

/// A provisioned JDK install, ready to serve test artifacts.
#[derive(Debug, Clone)]
pub struct Jdk {
    home: PathBuf,
    feature_version: u32,
}

impl Jdk {
    /// The JDK home — the directory holding `bin/`, `lib/`, `release`.
    pub fn home(&self) -> &Path {
        &self.home
    }

    /// The `jmods/` directory with one `.jmod` per platform module.
    pub fn jmods(&self) -> PathBuf {
        self.home.join("jmods")
    }

    /// A specific platform module, e.g. `jmod("java.base")`.
    pub fn jmod(&self, module: &str) -> PathBuf {
        self.jmods().join(format!("{module}.jmod"))
    }

    /// `lib/jrt-fs.jar` — a real jar that ships with every JDK.
    pub fn jrt_fs_jar(&self) -> PathBuf {
        self.home.join("lib/jrt-fs.jar")
    }

    pub fn feature_version(&self) -> u32 {
        self.feature_version
    }
}

/// Provision a Temurin JDK of the given feature version (e.g. `21`),
/// downloading and caching it on first use.
///
/// Panics with an actionable message on failure — this is test
/// infrastructure, not a library API.
pub fn jdk(feature_version: u32) -> Jdk {
    static PROVISIONED: OnceLock<Mutex<HashMap<u32, PathBuf>>> = OnceLock::new();
    let mut by_version = PROVISIONED
        .get_or_init(Default::default)
        .lock()
        .expect("provisioning mutex poisoned");

    let home = by_version
        .entry(feature_version)
        .or_insert_with(|| provision(feature_version))
        .clone();
    Jdk {
        home,
        feature_version,
    }
}

fn provision(version: u32) -> PathBuf {
    let (os, arch) = adoptium_platform();
    let url = format!(
        "https://api.adoptium.net/v3/binary/latest/{version}/ga/{os}/{arch}/jdk/hotspot/normal/eclipse"
    );

    let Some(tag) = latest_release_tag(&url) else {
        // Offline (or Adoptium is down): serve the newest cached build
        // rather than failing — the network is only required cold.
        return newest_cached(&cache_root(), version, os, arch).unwrap_or_else(|| {
            panic!("cannot reach Adoptium and no cached Temurin {version} exists")
        });
    };

    let slot = slot_path(version, os, arch, &tag);
    if let Some(home) = find_java_home(&slot) {
        return home;
    }

    let tmp = cache_root().join(format!(".tmp-{version}-{}", std::process::id()));
    fs::create_dir_all(&tmp).expect("create JDK cache temp dir");

    let archive = tmp.join("jdk.tar.gz");
    run(
        Command::new("curl")
            .args(["-fsSL", "--retry", "3", "-o"])
            .arg(&archive)
            .arg(&url),
        &format!("download Temurin {version} from {url}"),
    );
    run(
        Command::new("tar").arg("-xf").arg(&archive).arg("-C").arg(&tmp),
        "extract JDK archive",
    );
    fs::remove_file(&archive).expect("remove downloaded archive");

    // Atomic publish; if a parallel test binary won the race, use its
    // copy and discard ours.
    if fs::rename(&tmp, &slot).is_err() {
        fs::remove_dir_all(&tmp).expect("discard temp JDK after lost race");
    }

    let home = find_java_home(&slot).unwrap_or_else(|| {
        panic!(
            "downloaded Temurin {version} but found no JDK home (release + jmods) under {}",
            slot.display()
        )
    });
    prune_stale_slots(version, os, arch, &slot);
    home
}

/// Ask Adoptium what the latest GA build is without downloading it:
/// a HEAD request to `binary/latest` redirects through the GitHub
/// release asset URL, whose path carries the release tag. The chain
/// continues to an opaque CDN URL, so the tag must come from the
/// intermediate `Location` header, not the final URL. `None` when the
/// network is unavailable.
fn latest_release_tag(url: &str) -> Option<String> {
    let output = Command::new("curl")
        .args(["-fsIL", "--max-time", "15"])
        .arg(url)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    tag_from_response_headers(&String::from_utf8_lossy(&output.stdout))
}

/// Scan a `curl -sIL` header transcript for the redirect into a GitHub
/// release (`.../releases/download/<tag>/<asset>`) and extract the tag.
fn tag_from_response_headers(headers: &str) -> Option<String> {
    headers
        .lines()
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim().eq_ignore_ascii_case("location").then(|| value.trim())
        })
        .find(|location| location.contains("/releases/download/"))
        .and_then(release_tag_from_url)
}

/// `https://github.com/.../releases/download/jdk-21.0.11%2B9/OpenJDK21U-....tar.gz`
/// → `jdk-21.0.11+9` (second-to-last path segment, percent-decoded).
fn release_tag_from_url(url: &str) -> Option<String> {
    let mut segments = url.trim_end_matches('/').rsplit('/');
    let _asset = segments.next()?;
    let tag = segments.next()?;
    if tag.is_empty() {
        return None;
    }
    Some(tag.replace("%2B", "+").replace("%2b", "+"))
}

fn slot_prefix(version: u32, os: &str, arch: &str) -> String {
    format!("temurin-{version}-{os}-{arch}")
}

fn slot_path(version: u32, os: &str, arch: &str, tag: &str) -> PathBuf {
    cache_root().join(format!("{}-{tag}", slot_prefix(version, os, arch)))
}

/// Newest cached build for a feature version, by slot mtime (set when
/// the slot was renamed into place, i.e. download time).
fn newest_cached(cache_root: &Path, version: u32, os: &str, arch: &str) -> Option<PathBuf> {
    let prefix = slot_prefix(version, os, arch);
    let mut slots: Vec<_> = fs::read_dir(cache_root)
        .ok()?
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .strip_prefix(&prefix)
                .is_some_and(|rest| rest.starts_with('-'))
        })
        .collect();
    slots.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
    slots.into_iter().rev().find_map(|e| find_java_home(&e.path()))
}

/// Drop superseded builds of the same feature version — including
/// pre-patch-aware slots named without a release tag. Best effort;
/// a slot in use by a parallel run may survive until the next prune.
fn prune_stale_slots(version: u32, os: &str, arch: &str, current: &Path) {
    let prefix = slot_prefix(version, os, arch);
    let Ok(entries) = fs::read_dir(cache_root()) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path != current && entry.file_name().to_string_lossy().starts_with(&prefix) {
            let _ = fs::remove_dir_all(&path);
        }
    }
}

fn cache_root() -> PathBuf {
    if let Some(dir) = std::env::var_os("BEANS_TEST_JDK_CACHE") {
        return PathBuf::from(dir);
    }
    let home = std::env::var_os("HOME").expect("HOME not set; set BEANS_TEST_JDK_CACHE instead");
    PathBuf::from(home).join(".cache/beans/test-jdks")
}

/// Find the actual JDK home inside a cache slot. Temurin archives root
/// at `jdk-<ver>/`; on macOS the home is nested as `Contents/Home`.
/// Requiring `jmods/` next to `release` filters out JRE-style layouts.
fn find_java_home(slot: &Path) -> Option<PathBuf> {
    for entry in fs::read_dir(slot).ok()? {
        let root = entry.ok()?.path();
        for candidate in [root.clone(), root.join("Contents/Home")] {
            if candidate.join("release").is_file() && candidate.join("jmods").is_dir() {
                return Some(candidate);
            }
        }
    }
    None
}

fn adoptium_platform() -> (&'static str, &'static str) {
    let os = match std::env::consts::OS {
        "macos" => "mac",
        "linux" => "linux",
        other => panic!("no Adoptium platform mapping for OS {other:?}"),
    };
    let arch = match std::env::consts::ARCH {
        "aarch64" => "aarch64",
        "x86_64" => "x64",
        other => panic!("no Adoptium platform mapping for arch {other:?}"),
    };
    (os, arch)
}

fn run(command: &mut Command, what: &str) {
    let status = command
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn `{:?}` to {what}: {e}", command.get_program()));
    assert!(status.success(), "failed to {what}: {status}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_tag_comes_from_redirect_url() {
        assert_eq!(
            release_tag_from_url(
                "https://github.com/adoptium/temurin21-binaries/releases/download/\
                 jdk-21.0.11%2B9/OpenJDK21U-jdk_aarch64_mac_hotspot_21.0.11_9.tar.gz"
            )
            .as_deref(),
            Some("jdk-21.0.11+9")
        );
        // GA builds without a patch component decode the same way.
        assert_eq!(
            release_tag_from_url("https://example.com/download/jdk-21%2B35/asset.tar.gz")
                .as_deref(),
            Some("jdk-21+35")
        );
    }

    #[test]
    fn tag_is_taken_from_the_release_redirect_not_the_cdn_hop() {
        // Abbreviated transcript of `curl -sIL` against binary/latest:
        // Adoptium 307s to the GitHub release asset, which 302s onward
        // to an opaque CDN URL that must NOT be used for the tag.
        let headers = "\
HTTP/2 307\r
location: https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.11%2B9/OpenJDK21U-jdk_aarch64_mac_hotspot_21.0.11_9.tar.gz\r
\r
HTTP/2 302\r
Location: https://objects.githubusercontent.com/github-production-release-asset/602574963/abc?X-Amz-Signature=sig\r
\r
HTTP/2 200\r
content-type: application/octet-stream\r
";
        assert_eq!(
            tag_from_response_headers(headers).as_deref(),
            Some("jdk-21.0.11+9")
        );
        assert_eq!(tag_from_response_headers("HTTP/2 200\r\n"), None);
    }

    #[test]
    fn newest_cached_serves_the_most_recent_matching_build() {
        let root = std::env::temp_dir().join(format!("beans-test-jdks-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let make_slot = |name: &str| {
            let home = root.join(name).join("jdk/Contents/Home");
            fs::create_dir_all(home.join("jmods")).unwrap();
            fs::write(home.join("release"), "JAVA_VERSION=\"21\"\n").unwrap();
        };

        make_slot("temurin-21-mac-aarch64-jdk-21.0.10+7");
        std::thread::sleep(std::time::Duration::from_millis(25));
        make_slot("temurin-21-mac-aarch64-jdk-21.0.11+10");
        make_slot("temurin-17-mac-aarch64-jdk-17.0.10+7"); // other version

        let newest = newest_cached(&root, 21, "mac", "aarch64").expect("a cached build");
        assert!(
            newest.to_string_lossy().contains("21.0.11+10"),
            "picked {} instead of the newest 21 build",
            newest.display()
        );
        assert!(newest_cached(&root, 25, "mac", "aarch64").is_none());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn provisions_a_usable_jdk_21() {
        let jdk = jdk(21);

        assert!(jdk.home().join("bin/java").is_file());
        assert!(jdk.jmod("java.base").is_file());
        assert!(jdk.jrt_fs_jar().is_file());

        let release =
            fs::read_to_string(jdk.home().join("release")).expect("release file readable");
        assert!(
            release.contains("JAVA_VERSION=\"21"),
            "release file does not pin version 21: {release}"
        );
    }
}
