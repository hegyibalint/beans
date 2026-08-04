use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

/// The JDK a test reads.
///
/// A runtime image cannot be committed — the smallest `jlink` will build is
/// 17 MB — so anything that needs one reads a real JDK. Which JDK comes from
/// `mise.toml`, so a clone reads what a clone was given rather than whatever
/// happens to be on the machine. `MISE_JAVA_VERSION` names another for a
/// single run, and `mise.ci.toml` lists the sweep.
///
/// `mise` searches upward from the working directory, which `cargo test` sets
/// to the package root — every package here is inside the repository, so they
/// all reach the same pin.
pub fn home() -> &'static PathBuf {
    static HOME: OnceLock<PathBuf> = OnceLock::new();
    HOME.get_or_init(|| {
        let output = Command::new("mise")
            .args(["where", "java"])
            .output()
            .expect("`mise` should be on PATH; it is what provisions the JDKs");
        assert!(
            output.status.success(),
            "no JDK to read: run `mise install`\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        PathBuf::from(String::from_utf8_lossy(&output.stdout).trim())
    })
}

/// The one file a JDK contributes to a workspace: every system module, in one
/// container.
pub fn runtime_image() -> PathBuf {
    home().join("lib").join("modules")
}
