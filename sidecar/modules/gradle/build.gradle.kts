plugins {
    `java-library`
}

val modelProvider by configurations.creating {
    isCanBeConsumed = false
    isTransitive = false
}

dependencies {
    api(project(":gradle-model"))
    modelProvider(project(":gradle-model"))
    implementation(project(":core"))
    implementation(libs.gradle.tooling.api)
    runtimeOnly(libs.slf4j.simple)
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(libs.versions.java.get())
    }
}

tasks.processResources {
    from(modelProvider) {
        into("dev/blnt/beans/sidecar/gradle")
        rename { "model.jar" }
    }
}

val writeRuntimeClasspath = tasks.register("writeRuntimeClasspath") {
    inputs.files(configurations.runtimeClasspath)
    outputs.file(layout.buildDirectory.file("runtime-classpath.txt"))
    doLast {
        val paths = inputs.files.files.map { it.absolutePath }.sorted()
        outputs.files.singleFile.writeText(paths.joinToString("\n", postfix = "\n"))
    }
}

tasks.jar {
    dependsOn(writeRuntimeClasspath)
}
