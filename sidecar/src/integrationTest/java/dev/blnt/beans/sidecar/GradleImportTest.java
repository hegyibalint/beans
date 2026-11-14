package dev.blnt.beans.sidecar;

import dev.blnt.beans.sidecar.gradle.GradleAdapter;
import dev.blnt.beans.sidecar.gradle.ImportParams;
import dev.blnt.beans.sidecar.model.Classpath;
import dev.blnt.beans.sidecar.model.Location;
import dev.blnt.beans.sidecar.model.Module;
import dev.blnt.beans.sidecar.model.ResourceFilter;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.jar.JarOutputStream;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

class GradleImportTest {

    @Test
    void configured_source_directories_select_distinct_main_and_test_classpaths(@TempDir Path root)
            throws Exception {
        root = root.toRealPath();
        writeFixture(root);
        var modules = importFixture(root);
        var main = classpath(modules, root.resolve("mainSrc"));
        var test = classpath(modules, root.resolve("testSrc"));

        assertFalse(main.inputs().contains(file(root.resolve("testSrc/com/example/AppTest.java"))));
        assertTrue(test.entries().contains(file(root.resolve("build/classes/java/main"))));
        assertFalse(test.entries().contains(file(root.resolve("mainSrc/com/example/App.java"))));
        assertFalse(main.entries().contains(file(root.resolve("testSrc/com/example/AppTest.java"))));
    }

    @Test
    void a_classpath_includes_its_jdk_runtime_artifact(@TempDir Path root) throws Exception {
        root = root.toRealPath();
        writeFixture(root);
        var main = classpath(importFixture(root), root.resolve("mainSrc"));

        assertTrue(main.entries().stream().anyMatch(filter ->
                filter instanceof ResourceFilter.File file
                        && Path.of(file.root()).getFileName().toString().equals("modules")
                        && Files.isRegularFile(Path.of(file.root()))));
    }

    @Test
    void project_dependencies_preserve_compile_artifacts_and_source_associations(@TempDir Path root)
            throws Exception {
        root = root.toRealPath();
        // Describe a multi-project build with compile, test, and runtime-only dependencies.
        Files.writeString(root.resolve("settings.gradle.kts"), """
                rootProject.name = "multi"
                include("app", "lib")
                """);
        Files.writeString(root.resolve("build.gradle.kts"), """
                // The library plugin exposes class-directory variants for compilation.
                subprojects { apply(plugin = "java-library") }
                """);
        Files.createDirectories(root.resolve("app/src/main/java"));
        Files.createDirectories(root.resolve("app/src/test/java"));
        Files.createDirectories(root.resolve("lib/src/main/java"));
        Files.writeString(root.resolve("app/src/main/java/Use.java"), "class Use {}");
        Files.writeString(root.resolve("app/src/test/java/UseTest.java"), "class UseTest {}");
        Files.writeString(root.resolve("lib/src/main/java/Target.java"), "class Target {}");
        Files.writeString(root.resolve("app/build.gradle.kts"), """
                dependencies {
                    implementation(project(":lib"))
                    implementation(files("compile.jar"))
                    runtimeOnly(files("runtime.jar"))
                    testImplementation(files("test.jar"))
                }
                """);
        for (String name : List.of("compile.jar", "runtime.jar", "test.jar")) {
            try (var jar = new JarOutputStream(Files.newOutputStream(root.resolve("app/" + name)))) {
            }
        }

        // Inspect the shared module data rather than Gradle-specific unit identities.
        var modules = importFixture(root);
        assertEquals(2, modules.size());
        var main = classpath(modules, root.resolve("app/src/main/java"));
        var test = classpath(modules, root.resolve("app/src/test/java"));
        var library = classpath(modules, root.resolve("lib/src/main/java"));
        var target = file(root.resolve("lib/src/main/java/Target.java"));
        var classes = root.resolve("lib/build/classes/java/main");
        assertTrue(main.entries().contains(file(classes)));
        assertFalse(main.entries().contains(target));
        assertTrue(main.entries().contains(file(root.resolve("app/compile.jar"))));
        assertFalse(main.entries().contains(file(root.resolve("app/test.jar"))));
        assertFalse(main.entries().contains(file(root.resolve("app/runtime.jar"))));
        assertTrue(test.entries().contains(file(root.resolve("app/build/classes/java/main"))));
        assertFalse(test.entries().contains(file(root.resolve("app/src/main/java/Use.java"))));
        assertTrue(test.entries().contains(file(classes)));
        assertFalse(test.entries().contains(target));
        assertTrue(test.entries().contains(file(root.resolve("app/test.jar"))));
        assertFalse(test.entries().contains(file(root.resolve("app/runtime.jar"))));
        assertFalse(library.entries().contains(file(root.resolve("app/build/classes/java/main"))));
        var directorySources = modules.stream().flatMap(module -> module.artifactSources().stream())
                .filter(sources -> sources.sources().contains(target)).findFirst().orElseThrow();
        assertTrue(directorySources.artifacts().contains(Location.file(classes.toString())));

        // Packaged project dependencies retain the separate JAR-to-source association.
        Files.writeString(root.resolve("build.gradle.kts"), "subprojects { apply(plugin = \"java\") }\n");
        var packagedModules = importFixture(root);
        var packaged = classpath(packagedModules, root.resolve("app/src/main/java"));
        Path jar = root.resolve("lib/build/libs/lib.jar");
        assertTrue(packaged.entries().contains(file(jar)));
        assertFalse(packaged.entries().contains(target));
        var association = packagedModules.stream().flatMap(module -> module.artifactSources().stream())
                .filter(sources -> sources.sources().contains(target)).findFirst().orElseThrow();
        assertTrue(association.artifacts().contains(Location.file(jar.toString())));
        assertTrue(association.artifacts().contains(Location.file(
                root.resolve("lib/build/classes/java/main").toString())));
        assertFalse(Files.exists(jar));
    }

    @Test
    void an_import_narrates_its_progress(@TempDir Path root) throws Exception {
        root = root.toRealPath();
        writeFixture(root);
        List<String> progress = new ArrayList<>();
        new GradleAdapter().importWorkspace(params(root), progress::add);
        assertFalse(progress.isEmpty());
    }

    private static void writeFixture(Path root) throws Exception {
        Files.writeString(root.resolve("settings.gradle.kts"), "rootProject.name = \"it-fixture\"\n");
        Files.writeString(root.resolve("build.gradle.kts"), """
                plugins { java }
                sourceSets {
                    main { java.setSrcDirs(listOf("mainSrc")) }
                    test { java.setSrcDirs(listOf("testSrc")) }
                }
                """);
        Files.createDirectories(root.resolve("mainSrc/com/example"));
        Files.writeString(root.resolve("mainSrc/com/example/App.java"),
                "package com.example; public class App {}\n");
        Files.createDirectories(root.resolve("testSrc/com/example"));
        Files.writeString(root.resolve("testSrc/com/example/AppTest.java"),
                "package com.example; public class AppTest {}\n");
    }

    private static List<Module> importFixture(Path root) {
        return new GradleAdapter().importWorkspace(params(root), progress -> {});
    }

    private static ImportParams params(Path root) {
        String home = System.getProperty("test.gradleHome");
        assertNotNull(home, "test.gradleHome must be set by the build");
        return new ImportParams(root.toString(), home);
    }

    private static Classpath classpath(List<Module> modules, Path input) {
        return modules.stream().flatMap(module -> module.classpaths().stream())
                .filter(classpath -> classpath.inputs().stream().anyMatch(filter ->
                        filter instanceof ResourceFilter.File file
                                && Path.of(file.root()).startsWith(input)))
                .findFirst()
                .orElseThrow(() -> new AssertionError("No classpath for " + input));
    }

    private static ResourceFilter file(Path path) {
        return new ResourceFilter.File(path.toAbsolutePath().normalize().toString());
    }
}
