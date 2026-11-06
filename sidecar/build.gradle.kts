// Production assembly and JVM integration tests.
plugins {
    java
    `jvm-test-suite`
}

dependencies {
    runtimeOnly(project(":core"))
    runtimeOnly(project(":gradle"))
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
        register<JvmTestSuite>("integrationTest") {
            useJUnitJupiter(libs.versions.junit)
            dependencies {
                implementation(libs.jackson.databind)
                implementation(project(":core"))
                implementation(project(":gradle"))
            }
            targets {
                all {
                    testTask.configure {
                        dependsOn(tasks.jar)
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
