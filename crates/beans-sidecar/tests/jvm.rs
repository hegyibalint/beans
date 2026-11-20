use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use beans_core::{location::Location, resource_filter::ResourceFilter};
use beans_engine::Engine;
use beans_sidecar::Sidecar;
use beans_testing::{TempDir, support::jdks};

#[test]
#[ignore = "requires a mise Temurin 17 JDK and prebuilt sidecar JARs (gradlew assemble)"]
fn pings_a_persistent_jvm_without_module_stdout_corrupting_the_protocol() {
    let java_home = sidecar_jdk();
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
        .arg(env::join_paths(classpath(&root, false)).unwrap())
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

#[test]
#[ignore = "requires a mise Temurin 17 JDK, GRADLE_HOME and prebuilt sidecar JARs"]
fn gradle_import_registers_modules_and_preserves_them_on_failure() {
    let java_home = sidecar_jdk();
    let gradle_home = PathBuf::from(env::var_os("GRADLE_HOME").expect("set GRADLE_HOME"));
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("sidecar");
    let files = TempDir::new();
    let project = fs::canonicalize(files.root()).unwrap();
    fs::write(
        project.join("settings.gradle"),
        "rootProject.name = 'import-test'\n",
    )
    .unwrap();
    fs::write(project.join("build.gradle"), "plugins { id 'groovy' }\n").unwrap();
    for (directory, name) in [
        ("src/main/java", "Use.java"),
        ("src/main/groovy", "GroovyUse.groovy"),
        ("src/test/java", "UseTest.java"),
    ] {
        fs::create_dir_all(project.join(directory)).unwrap();
        fs::write(project.join(directory).join(name), "class Example {}\n").unwrap();
    }
    let engine = Arc::new(Engine::new(&project).unwrap());
    let mut command = Command::new(
        java_home
            .join("bin")
            .join(format!("java{}", env::consts::EXE_SUFFIX)),
    );
    command
        .env("JAVA_HOME", &java_home)
        .arg("-cp")
        .arg(env::join_paths(classpath(&root, true)).unwrap())
        .arg("dev.blnt.beans.sidecar.Main")
        .stderr(fs::File::create(files.path("import-stderr.log")).unwrap());
    let mut sidecar = Sidecar::default();
    sidecar.start(command).unwrap();

    // Complete a real RPC import, preserving classpaths and artifact-to-source associations.
    sidecar
        .import_gradle(&project, Some(&gradle_home), Arc::downgrade(&engine))
        .unwrap()
        .join()
        .unwrap()
        .unwrap();
    let source = Location::file(project.join("src/main/java/Use.java"));
    let modules = engine.modules().unwrap();
    assert_eq!(modules.len(), 1);
    assert_eq!(modules[0].classpaths.len(), 2);
    let java = ResourceFilter::file(project.join("src/main/java/Use.java"));
    let association = modules[0]
        .artifact_sources
        .iter()
        .find(|association| association.sources.contains(&java))
        .unwrap();
    assert!(
        association
            .artifacts
            .contains(&Location::file(project.join("build/libs/import-test.jar")))
    );
    assert!(association.sources.contains(&ResourceFilter::file(
        project.join("src/main/groovy/GroovyUse.groovy")
    )));
    let expected = serde_json::to_value(&modules).unwrap();
    let classpaths = &modules[0].classpaths;
    let main: Vec<_> = classpaths
        .iter()
        .filter(|classpath| classpath.inputs.iter().any(|input| input.contains(&source)))
        .collect();
    assert_eq!(main.len(), 1);
    assert!(main[0].inputs.contains(&ResourceFilter::file(
        project.join("src/main/groovy/GroovyUse.groovy")
    )));
    let test = ResourceFilter::file(project.join("src/test/java/UseTest.java"));
    assert_eq!(
        classpaths
            .iter()
            .filter(|classpath| classpath.inputs.contains(&test))
            .count(),
        1
    );
    assert!(!project.join("build/classes").exists());

    // A failed import cannot replace the last successful snapshot or break the session.
    fs::write(
        project.join("build.gradle"),
        "throw new GradleException('intentional import failure')\n",
    )
    .unwrap();
    assert!(
        sidecar
            .import_gradle(&project, Some(&gradle_home), Arc::downgrade(&engine))
            .unwrap()
            .join()
            .unwrap()
            .is_err()
    );
    assert_eq!(
        serde_json::to_value(engine.modules().unwrap()).unwrap(),
        expected
    );
    sidecar.ping().unwrap();

    // A later successful empty import clears this build's snapshot.
    fs::write(project.join("build.gradle"), "").unwrap();
    sidecar
        .import_gradle(&project, Some(&gradle_home), Arc::downgrade(&engine))
        .unwrap()
        .join()
        .unwrap()
        .unwrap();
    assert!(engine.modules().unwrap().is_empty());
    sidecar.stop().unwrap();
}

fn sidecar_jdk() -> PathBuf {
    jdks::list("temurin-17")
        .unwrap()
        .into_iter()
        .next()
        .expect("install a Temurin 17 JDK with mise before running this test")
}

fn classpath(root: &Path, gradle: bool) -> Vec<PathBuf> {
    let mut projects = vec![
        ("core", "core"),
        ("fixtures/stdout-screamer", "stdout-screamer"),
    ];
    if gradle {
        projects.push(("modules/gradle", "gradle"));
    }
    let mut entries = Vec::new();
    for &(directory, name) in &projects {
        entries.push(root.join(directory).join(format!("build/libs/{name}.jar")));
    }
    for &(directory, _) in &projects {
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
