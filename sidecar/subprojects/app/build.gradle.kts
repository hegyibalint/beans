// Assembly only: no sources of its own, just the one artifact the Rust client
// launches and the tests that speak to it the way the client will.
plugins {
    java
    `jvm-test-suite`
}

dependencies {
    runtimeOnly(project(":core"))
    // Found through ServiceLoader at runtime; nothing compiles against it.
    runtimeOnly(project(":adapter-gradle"))
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(17)
    }
}

tasks.jar {
    // The mapped list below is plain values, so it carries no task dependency
    // of its own and would happily zip subproject jars that do not exist yet.
    dependsOn(configurations.runtimeClasspath)
    archiveBaseName = "beans-sidecar"
    manifest {
        attributes["Main-Class"] = "dev.blnt.beans.sidecar.Main"
        attributes["Implementation-Title"] = "beans-sidecar"
    }
    from(configurations.runtimeClasspath.get().map { if (it.isDirectory) it else zipTree(it) })
    duplicatesStrategy = DuplicatesStrategy.EXCLUDE
    exclude("META-INF/*.SF", "META-INF/*.DSA", "META-INF/*.RSA")
}

testing {
    suites {
        // Spawns the real jar and speaks the real protocol over stdio.
        register<JvmTestSuite>("integrationTest") {
            useJUnitJupiter(libs.versions.junit)
            dependencies {
                implementation(libs.jackson.databind)
            }
            targets {
                all {
                    testTask.configure {
                        dependsOn(tasks.jar)
                        systemProperty(
                            "test.sidecarJar",
                            tasks.jar.get().archiveFile.get().asFile.absolutePath,
                        )
                        // The import fixture has no wrapper; the Gradle running
                        // this build serves as the installation under test.
                        gradle.gradleHomeDir?.let {
                            systemProperty("test.gradleHome", it.absolutePath)
                        }
                        testLogging {
                            events("passed", "failed", "skipped")
                        }
                    }
                }
            }
        }
    }
}

tasks.check {
    dependsOn(testing.suites.named("integrationTest"))
}
