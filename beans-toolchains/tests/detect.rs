//! Detection-pipeline tests against synthetic JDK trees.
//!
//! Each test builds fake Java homes in a temp dir (a `bin/java` marker
//! plus an optional `release` file) and drives the pipeline through
//! the same seams the real entry point uses: suppliers → candidates →
//! [`beans_toolchains::detect_from`] → [`beans_toolchains::select`].

use std::fs;
use std::path::{Path, PathBuf};

use beans_toolchains::suppliers::{self, Candidate};
use beans_toolchains::{ToolchainSpec, detect_from, select};

/// Unique temp dir per test; best-effort cleanup on drop.
struct TempTree(PathBuf);

impl TempTree {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "beans-toolchains-test-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    /// Create a fake Java home at `rel`, optionally with a release file.
    fn fake_jdk(&self, rel: &str, release: Option<&str>, with_javac: bool) -> PathBuf {
        let home = self.0.join(rel);
        let bin = home.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("java"), "").unwrap();
        if with_javac {
            fs::write(bin.join("javac"), "").unwrap();
        }
        if let Some(content) = release {
            fs::write(home.join("release"), content).unwrap();
        }
        home
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.0.join(rel)
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn release(version: &str, vendor: &str) -> String {
    format!("JAVA_VERSION=\"{version}\"\nIMPLEMENTOR=\"{vendor}\"\nOS_ARCH=\"aarch64\"\n")
}

#[test]
fn detects_probes_and_dedupes() {
    let t = TempTree::new("dedupe");
    let jdk21 = t.fake_jdk(
        "sdkman/21.0.2-tem",
        Some(&release("21.0.2", "Eclipse Adoptium")),
        true,
    );

    // The same home reported by two suppliers, one via a symlink.
    let link = t.path("current");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&jdk21, &link).unwrap();
    #[cfg(not(unix))]
    let link = jdk21.clone();

    let raw = vec![
        Candidate {
            path: jdk21.clone(),
            source: "SDKMAN".into(),
        },
        Candidate {
            path: link,
            source: "JAVA_HOME".into(),
        },
        Candidate {
            path: t.path("nonexistent"),
            source: "PATH".into(),
        },
    ];

    let installs = detect_from(&raw);
    assert_eq!(installs.len(), 1, "symlink and direct path must dedupe");
    let inst = &installs[0];
    assert_eq!(
        inst.sources,
        vec!["SDKMAN".to_string(), "JAVA_HOME".to_string()]
    );
    let meta = inst
        .metadata
        .as_ref()
        .expect("release file probed during detect");
    assert_eq!(meta.major, 21);
    assert_eq!(meta.version, "21.0.2");
    assert_eq!(meta.vendor.as_deref(), Some("Eclipse Adoptium"));
    assert!(inst.is_jdk());
}

#[test]
fn unwraps_macos_bundles_and_nested_archives() {
    let t = TempTree::new("unwrap");
    // macOS bundle shape: <dir>/Contents/Home/bin/java.
    t.fake_jdk(
        "JavaVirtualMachines/temurin-17.jdk/Contents/Home",
        Some(&release("17.0.10", "Eclipse Adoptium")),
        true,
    );
    // Gradle ~/.gradle/jdks shape: supplier sees the outer dir, the
    // actual JDK is one level down.
    t.fake_jdk(
        "gradle-jdks/azul-21-os_x/zulu-21.jdk",
        Some(&release("21.0.1", "Azul Systems, Inc.")),
        true,
    );

    let raw = vec![
        Candidate {
            path: t.path("JavaVirtualMachines/temurin-17.jdk"),
            source: "macOS JavaVirtualMachines".into(),
        },
        Candidate {
            path: t.path("gradle-jdks/azul-21-os_x"),
            source: "Gradle ~/.gradle/jdks".into(),
        },
    ];

    let installs = detect_from(&raw);
    assert_eq!(installs.len(), 2);
    let majors: Vec<u32> = installs
        .iter()
        .map(|i| i.metadata.as_ref().unwrap().major)
        .collect();
    assert!(majors.contains(&17) && majors.contains(&21));
}

#[test]
fn selection_prefers_jdk_then_highest_version() {
    let t = TempTree::new("select");
    t.fake_jdk("jre-21", Some(&release("21.0.2", "V")), false);
    t.fake_jdk("jdk-17", Some(&release("17.0.10", "V")), true);
    t.fake_jdk("jdk-11", Some(&release("11.0.22", "V")), true);

    let raw: Vec<Candidate> = ["jre-21", "jdk-17", "jdk-11"]
        .iter()
        .map(|rel| Candidate {
            path: t.path(rel),
            source: "test".into(),
        })
        .collect();

    let mut installs = detect_from(&raw);

    // Any JVM: the JRE has the highest version but loses to a JDK.
    let best = select(&mut installs, &ToolchainSpec::default()).unwrap();
    assert!(best.java_home.ends_with("jdk-17"), "JDK beats newer JRE");

    // Exact major.
    let spec = ToolchainSpec {
        exact_major: Some(11),
        ..Default::default()
    };
    let best = select(&mut installs, &spec).unwrap();
    assert!(best.java_home.ends_with("jdk-11"));

    // Min major that only the JRE satisfies; require_jdk must refuse it.
    let spec = ToolchainSpec {
        min_major: Some(21),
        require_jdk: true,
        ..Default::default()
    };
    assert!(select(&mut installs, &spec).is_none());

    // Without require_jdk, the JRE is acceptable.
    let spec = ToolchainSpec {
        min_major: Some(21),
        ..Default::default()
    };
    let best = select(&mut installs, &spec).unwrap();
    assert!(best.java_home.ends_with("jre-21"));
}

#[test]
fn maven_toolchains_parses_jdk_homes_with_env_substitution() {
    let xml = r#"
        <toolchains>
          <toolchain>
            <type>jdk</type>
            <configuration><jdkHome>/opt/java/jdk-17</jdkHome></configuration>
          </toolchain>
          <toolchain>
            <type>jdk</type>
            <configuration><jdkHome>${env.MY_JDK}/home</jdkHome></configuration>
          </toolchain>
        </toolchains>
    "#;
    let env = |k: &str| (k == "MY_JDK").then(|| "/custom".to_string());
    let cands = suppliers::maven_toolchains(xml, &env);
    let paths: Vec<&Path> = cands.iter().map(|c| c.path.as_path()).collect();
    assert_eq!(
        paths,
        vec![Path::new("/opt/java/jdk-17"), Path::new("/custom/home")]
    );
}

#[test]
fn release_less_home_is_kept_unprobed_until_selection() {
    let t = TempTree::new("unprobed");
    t.fake_jdk("mystery-jdk", None, true);

    let raw = vec![Candidate {
        path: t.path("mystery-jdk"),
        source: "test".into(),
    }];
    let mut installs = detect_from(&raw);
    assert_eq!(installs.len(), 1);
    assert!(
        installs[0].metadata.is_none(),
        "no release file → unprobed after detect"
    );

    // Selection exec-probes it; the fake bin/java fails to run, so the
    // candidate is skipped rather than crashing the pipeline.
    let best = select(&mut installs, &ToolchainSpec::default());
    assert!(best.is_none(), "unprobeable candidate must be dropped");
}
