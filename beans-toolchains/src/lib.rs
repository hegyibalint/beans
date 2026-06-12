//! beans-toolchains — locate JAVA_HOME-compatible directories.
//!
//! The JVM is a *data pipe* for beans, never a prerequisite: the engine
//! serves from its graph immediately, and tooling that needs a JVM
//! (the jmod reader's JDK lookup, the Gradle/Maven import sidecar)
//! catches up whenever one is found. This crate is the finding part.
//!
//! Architecture is borrowed from Gradle's toolchain detection (Apache-2,
//! `org.gradle.jvm.toolchain.internal`), trimmed to what beans needs:
//!
//! 1. **Suppliers** ([`suppliers`]) fan out over the places JVMs live:
//!    `JAVA_HOME`/PATH, version managers (SDKMAN, asdf, mise), tool
//!    caches (IntelliJ `~/.jdks`, Gradle `~/.gradle/jdks`, Maven
//!    `toolchains.xml`), and OS conventions.
//! 2. **Normalization** ([`candidate`]) canonicalizes symlinks, unwraps
//!    macOS `Contents/Home` and single-nested archive dirs, requires
//!    `bin/java`, and dedupes by canonical path (merging sources).
//! 3. **Probing** ([`probe`]) parses `<home>/release` — present in
//!    every JDK 9+ — and falls back to a bounded `java
//!    -XshowSettings:properties` exec only when a candidate without a
//!    release file is actually being considered by selection.
//! 4. **Selection** ([`select`]) matches a [`ToolchainSpec`] and orders
//!    JDK-over-JRE, then highest version, then stable path.
//!
//! The API is synchronous; never-block is the consumer's contract (run
//! detection off the latency-sensitive thread). Results are not
//! persisted — detection is a fistful of directory reads; a disk cache
//! is a measured-later optimization.

mod candidate;
mod probe;
mod select;
pub mod suppliers;

use std::path::PathBuf;

pub use probe::JvmMetadata;
pub use select::ToolchainSpec;

/// One JAVA_HOME-compatible directory, post normalization.
#[derive(Debug, Clone)]
pub struct JavaInstallation {
    /// Canonical home: `bin/java` exists beneath it.
    pub java_home: PathBuf,
    /// Every supplier that reported this home (post-dedup).
    pub sources: Vec<String>,
    /// Probed metadata. `None` until probed; release-file probing
    /// happens during [`detect`], the exec fallback lazily during
    /// [`select`](JavaInstallation::ensure_metadata).
    pub metadata: Option<JvmMetadata>,
}

impl JavaInstallation {
    /// True iff this home ships a compiler (`bin/javac`) — the
    /// JDK-vs-JRE test, checked on the filesystem, not probed.
    pub fn is_jdk(&self) -> bool {
        let bin = self.java_home.join("bin");
        bin.join("javac").exists() || bin.join("javac.exe").exists()
    }

    /// Probe metadata if not yet known: release file first, exec
    /// fallback second. Returns the metadata if either succeeds.
    pub fn ensure_metadata(&mut self) -> Option<&JvmMetadata> {
        if self.metadata.is_none() {
            self.metadata = probe::from_release_file(&self.java_home)
                .or_else(|| probe::from_exec(&self.java_home));
        }
        self.metadata.as_ref()
    }
}

/// Run every supplier against the real host environment, normalize,
/// dedupe, and probe release files. Cheap (directory reads + small
/// file parses); no process is spawned. Candidates whose release file
/// is missing or unparseable surface with `metadata: None` and are
/// exec-probed lazily by [`select`].
pub fn detect() -> Vec<JavaInstallation> {
    detect_from(&suppliers::real_candidates())
}

/// Detection pipeline over an explicit candidate list — the seam tests
/// use to run against fixture trees.
pub fn detect_from(raw: &[suppliers::Candidate]) -> Vec<JavaInstallation> {
    let mut installs = candidate::normalize_and_dedup(raw);
    for inst in &mut installs {
        inst.metadata = probe::from_release_file(&inst.java_home);
    }
    installs
}

/// Pick the best installation matching `spec`, exec-probing unprobed
/// candidates only as they are considered. See [`ToolchainSpec`].
pub fn select<'a>(
    installs: &'a mut [JavaInstallation],
    spec: &ToolchainSpec,
) -> Option<&'a JavaInstallation> {
    select::best_match(installs, spec)
}
