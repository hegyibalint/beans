plugins {
    `java-library`
}

dependencies {
    api(libs.jackson.databind)
    // The Tooling API and friends log through SLF4J; RpcLoggerServiceProvider
    // turns those events into protocol notifications instead of dropping them.
    api(libs.slf4j.api)
}

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(17)
    }
}
