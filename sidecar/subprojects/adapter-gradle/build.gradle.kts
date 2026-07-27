plugins {
    `java-library`
}

dependencies {
    implementation(project(":core"))
    implementation(libs.gradle.tooling.api)
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(17)
    }
}
