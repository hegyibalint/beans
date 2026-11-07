plugins {
    `java-library`
}

dependencies {
    compileOnly(gradleApi())
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(17)
    }
}

// These classes also run in the target build's daemon, not just our sidecar.
tasks.compileJava {
    options.release = 8
}
