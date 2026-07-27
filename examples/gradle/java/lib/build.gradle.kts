plugins {
    java
    idea
}

dependencies {
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

// Stands in for an annotation processor: a source root that only exists after
// a build, so an importer has to report it and a fresh clone has to cope
// without it.
val generateBuildInfo by tasks.registering {
    val outputDir = layout.buildDirectory.dir("generated/sources/buildInfo/java/main")
    outputs.dir(outputDir)
    doLast {
        val packageDir = outputDir.get().asFile.resolve("com/example/lib")
        packageDir.mkdirs()
        packageDir.resolve("BuildInfo.java").writeText(
            """
            package com.example.lib;

            public final class BuildInfo {
                public static final String VERSION = "1.0";

                private BuildInfo() {}
            }
            """.trimIndent(),
        )
    }
}

sourceSets {
    main {
        java.srcDir(generateBuildInfo)
    }
}

idea {
    module {
        generatedSourceDirs.add(
            layout.buildDirectory.dir("generated/sources/buildInfo/java/main").get().asFile,
        )
    }
}

tasks.test {
    useJUnitPlatform()
}
