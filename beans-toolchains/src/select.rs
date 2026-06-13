//! Selection: match a [`ToolchainSpec`] against detected installations.
//!
//! Ordering follows Gradle's `JvmInstallationMetadataComparator`,
//! trimmed: JDK over JRE, then highest version, then path as a stable
//! tiebreak. Candidates without release-file metadata are exec-probed
//! here — lazily, only when they are actually considered — and dropped
//! if the probe fails.

use crate::{JavaInstallation, JvmMetadata};

/// What the consumer needs. Empty spec = "any working JVM".
#[derive(Debug, Clone, Default)]
pub struct ToolchainSpec {
    /// Lowest acceptable feature release (inclusive).
    pub min_major: Option<u32>,
    /// Exact feature release; takes precedence over `min_major`.
    pub exact_major: Option<u32>,
    /// Require `bin/javac` (a JDK, not a JRE). The jmod reader and any
    /// compile-shaped sidecar work needs this; a launcher may not.
    pub require_jdk: bool,
}

impl ToolchainSpec {
    fn matches(&self, meta: &JvmMetadata, is_jdk: bool) -> bool {
        if self.require_jdk && !is_jdk {
            return false;
        }
        if let Some(exact) = self.exact_major {
            return meta.major == exact;
        }
        if let Some(min) = self.min_major {
            return meta.major >= min;
        }
        true
    }
}

pub fn best_match<'a>(
    installs: &'a mut [JavaInstallation],
    spec: &ToolchainSpec,
) -> Option<&'a JavaInstallation> {
    let mut best: Option<usize> = None;
    for i in 0..installs.len() {
        if installs[i].ensure_metadata().is_none() {
            continue;
        }
        let is_jdk = installs[i].is_jdk();
        let meta = installs[i].metadata.as_ref().unwrap();
        if !spec.matches(meta, is_jdk) {
            continue;
        }
        best = match best {
            None => Some(i),
            Some(b) => {
                if better(&installs[i], &installs[b]) {
                    Some(i)
                } else {
                    Some(b)
                }
            }
        };
    }
    best.map(|i| &installs[i])
}

/// True iff `a` outranks `b`: JDK first, then higher major, then the
/// full version compared by numeric segments (`25.0.10` > `25.0.9`,
/// which string comparison gets wrong), then path for determinism.
fn better(a: &JavaInstallation, b: &JavaInstallation) -> bool {
    let (ma, mb) = (a.metadata.as_ref().unwrap(), b.metadata.as_ref().unwrap());
    (
        a.is_jdk(),
        ma.major,
        version_key(&ma.version),
        std::cmp::Reverse(&a.java_home),
    ) > (
        b.is_jdk(),
        mb.major,
        version_key(&mb.version),
        std::cmp::Reverse(&b.java_home),
    )
}

/// Numeric segments of a version string; non-numeric tails rank as 0.
fn version_key(version: &str) -> Vec<u32> {
    version
        .split(['.', '_', '+', '-'])
        .map(|seg| seg.parse().unwrap_or(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::version_key;

    #[test]
    fn version_key_orders_numerically_not_lexically() {
        assert!(version_key("25.0.10") > version_key("25.0.9"));
        assert!(version_key("21.0.2+13") > version_key("21.0.2"));
        assert!(version_key("1.8.0_392") < version_key("1.8.0_402"));
    }
}
