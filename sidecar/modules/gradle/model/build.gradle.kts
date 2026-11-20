plugins {
    `java-library`
}

dependencies {
    compileOnly(gradleApi())
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(libs.versions.java.get())
    }
}

// These classes also run in the target build's daemon, not just our sidecar.
tasks.compileJava {
    options.release = 8
}
