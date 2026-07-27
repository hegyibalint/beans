plugins {
    java
}

dependencies {
    implementation(project(":lib"))
    implementation("org.apache.commons:commons-lang3:3.17.0")
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

sourceSets {
    // Configured rather than conventional, so an importer that guessed
    // Gradle's defaults would come up empty here.
    main {
        java.setSrcDirs(listOf("sources/java"))
    }
    // A third source set: Gradle has three compilation scopes in this project,
    // and the stock IDEA model only knows production and test.
    create("integrationTest") {
        compileClasspath += sourceSets["main"].output
        runtimeClasspath += output + compileClasspath
    }
}

tasks.test {
    useJUnitPlatform()
}
