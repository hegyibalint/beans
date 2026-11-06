use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use beans_sidecar::Sidecar;
use beans_testing::{TempDir, support::jdks};

#[test]
#[ignore = "requires a mise Temurin 17 JDK and prebuilt sidecar JARs (gradlew assemble)"]
fn pings_a_persistent_jvm_without_module_stdout_corrupting_the_protocol() {
    let java_home = jdks::list("temurin-17")
        .unwrap()
        .into_iter()
        .next()
        .expect("install a Temurin 17 JDK with mise before running this test");
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("sidecar");
    let files = TempDir::new();
    let stderr = files.path("stderr.log");
    let mut command = Command::new(
        java_home
            .join("bin")
            .join(format!("java{}", env::consts::EXE_SUFFIX)),
    );
    command
        .env("JAVA_HOME", &java_home)
        .arg("-cp")
        .arg(env::join_paths(classpath(&root)).unwrap())
        .arg("dev.blnt.beans.sidecar.Main")
        .stderr(fs::File::create(&stderr).unwrap());

    let mut sidecar = Sidecar::default();
    sidecar.start(command).unwrap();
    sidecar.ping().unwrap();
    sidecar.ping().unwrap();
    sidecar.stop().unwrap();

    let noise = fs::read_to_string(stderr).unwrap();
    assert!(noise.contains("stdout-screamer loaded"), "{noise}");
    assert!(
        noise.contains(r#"{"jsonrpc":"2.0","id":1,"result":"not-a-pong"}"#),
        "{noise}"
    );
}

fn classpath(root: &Path) -> Vec<PathBuf> {
    let projects = [
        ("core", "core"),
        ("fixtures/stdout-screamer", "stdout-screamer"),
    ];
    let mut entries = Vec::new();
    for (directory, name) in projects {
        entries.push(root.join(directory).join(format!("build/libs/{name}.jar")));
    }
    for (directory, _) in projects {
        let manifest = root.join(directory).join("build/runtime-classpath.txt");
        let dependencies = fs::read_to_string(&manifest).unwrap_or_else(|error| {
            panic!("build sidecar JARs first: {}: {error}", manifest.display())
        });
        for path in dependencies.lines().filter(|line| !line.is_empty()) {
            entries.push(PathBuf::from(path));
        }
    }
    let mut seen = HashSet::new();
    entries.retain(|path| seen.insert(path.clone()));
    for entry in &entries {
        assert!(
            entry.is_file(),
            "missing classpath entry: {}",
            entry.display()
        );
    }
    entries
}
