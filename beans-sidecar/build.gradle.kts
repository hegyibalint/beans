// The beans sidecar: one JVM process serving every JVM-bound duty over
// a JSON-Lines stdio protocol (ADR-0031). Single module for now; split
// into core + per-tool modules when a second build tool lands.
plugins {
    java
    `jvm-test-suite`
}

repositories {
    mavenCentral()
    // The Tooling API is published to Gradle's own repository.
    maven(url = "https://repo.gradle.org/gradle/libs-releases")
}

dependencies {
    implementation("org.gradle:gradle-tooling-api:9.2.0")
    implementation("com.fasterxml.jackson.core:jackson-databind:2.18.2")
    // TAPI logs through SLF4J; our RpcLoggerServiceProvider turns every
    // log event into a {"method":"log"} notification on the protocol
    // channel instead of discarding it.
    implementation("org.slf4j:slf4j-api:2.0.16")
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(17)
    }
}

// Fat jar: the sidecar ships as one artifact the Rust client can launch
// with nothing but a located JVM.
tasks.jar {
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
        // Unit tests: fast, in-process, no fixtures beyond the classes.
        val test by getting(JvmTestSuite::class) {
            useJUnitJupiter()
        }

        // Integration tests: spawn the real fat jar and speak the real
        // protocol over stdio. Need the jar and a Gradle installation
        // for the import fixture — the one running this build serves.
        val integrationTest by registering(JvmTestSuite::class) {
            useJUnitJupiter()
            dependencies {
                implementation("com.fasterxml.jackson.core:jackson-databind:2.18.2")
            }
            targets {
                all {
                    testTask.configure {
                        dependsOn(tasks.jar)
                        shouldRunAfter(test)
                        systemProperty(
                            "test.sidecarJar",
                            tasks.jar.get().archiveFile.get().asFile.absolutePath
                        )
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
