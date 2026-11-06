plugins {
    `java-library`
}

dependencies {
    implementation(project(":core"))
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(17)
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
