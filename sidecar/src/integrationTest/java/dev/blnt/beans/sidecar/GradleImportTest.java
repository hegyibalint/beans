package dev.blnt.beans.sidecar;

import dev.blnt.beans.sidecar.gradle.GradleAdapter;
import dev.blnt.beans.sidecar.gradle.ImportParams;
import dev.blnt.beans.sidecar.model.CompilationScope;
import dev.blnt.beans.sidecar.model.ResourceFilter;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.jar.JarOutputStream;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

class GradleImportTest {

    @Test
    void configured_source_directories_select_distinct_main_and_test_scopes(@TempDir Path root)
            throws Exception {
        root = root.toRealPath();
        writeFixture(root);
        var scopes = importFixture(root);
        var main = scope(scopes, root.resolve("mainSrc"));
        var test = scope(scopes, root.resolve("testSrc"));

        assertFalse(main.inputs().contains(file(root.resolve("testSrc/com/example/AppTest.java"))));
        assertTrue(test.dependencies().contains(file(root.resolve("mainSrc/com/example/App.java"))));
        assertFalse(main.dependencies().contains(file(root.resolve("testSrc/com/example/AppTest.java"))));
    }

    @Test
    void a_scope_includes_its_jdk_runtime_artifact(@TempDir Path root) throws Exception {
        root = root.toRealPath();
        writeFixture(root);
        var main = scope(importFixture(root), root.resolve("mainSrc"));

        assertTrue(main.dependencies().stream().anyMatch(filter ->
                filter instanceof ResourceFilter.File file
                        && Path.of(file.root()).getFileName().toString().equals("modules")
                        && Files.isRegularFile(Path.of(file.root()))));
    }

    @Test
    void project_and_library_dependencies_become_concrete_compile_filters(@TempDir Path root)
            throws Exception {
        root = root.toRealPath();
        // Describe a multi-project build with compile, test, and runtime-only dependencies.
        Files.writeString(root.resolve("settings.gradle.kts"), """
                rootProject.name = "multi"
                include("app", "lib")
                """);
        Files.writeString(root.resolve("build.gradle.kts"), """
                // The library plugin exposes class-directory variants for source linking.
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

        // Inspect the imported scopes rather than Gradle-specific unit identities.
        var scopes = importFixture(root);
        var main = scope(scopes, root.resolve("app/src/main/java"));
        var test = scope(scopes, root.resolve("app/src/test/java"));
        var library = scope(scopes, root.resolve("lib/src/main/java"));
        assertTrue(main.dependencies().contains(file(root.resolve("lib/src/main/java/Target.java"))));
        assertTrue(main.dependencies().contains(file(root.resolve("app/compile.jar"))));
        assertFalse(main.dependencies().contains(file(root.resolve("app/test.jar"))));
        assertFalse(main.dependencies().contains(file(root.resolve("app/runtime.jar"))));
        assertTrue(test.dependencies().contains(file(root.resolve("app/src/main/java/Use.java"))));
        assertTrue(test.dependencies().contains(file(root.resolve("lib/src/main/java/Target.java"))));
        assertTrue(test.dependencies().contains(file(root.resolve("app/test.jar"))));
        assertFalse(test.dependencies().contains(file(root.resolve("app/runtime.jar"))));
        assertFalse(library.dependencies().contains(file(root.resolve("app/src/main/java/Use.java"))));

        // Preserve packaged project dependencies without guessing which sources a JAR contains.
        Files.writeString(root.resolve("build.gradle.kts"), "subprojects { apply(plugin = \"java\") }\n");
        var packaged = scope(importFixture(root), root.resolve("app/src/main/java"));
        assertTrue(packaged.dependencies().contains(file(root.resolve("lib/build/libs/lib.jar"))));
        assertFalse(packaged.dependencies().contains(file(root.resolve("lib/src/main/java/Target.java"))));
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

    private static List<CompilationScope> importFixture(Path root) {
        return new GradleAdapter().importWorkspace(params(root), progress -> {});
    }

    private static ImportParams params(Path root) {
        String home = System.getProperty("test.gradleHome");
        assertNotNull(home, "test.gradleHome must be set by the build");
        return new ImportParams(root.toString(), home);
    }

    private static CompilationScope scope(List<CompilationScope> scopes, Path input) {
        return scopes.stream()
                .filter(scope -> scope.inputs().stream().anyMatch(filter ->
                        filter instanceof ResourceFilter.File file
                                && Path.of(file.root()).startsWith(input)))
                .findFirst()
                .orElseThrow(() -> new AssertionError("No scope for " + input));
    }

    private static ResourceFilter file(Path path) {
        return new ResourceFilter.File(path.toAbsolutePath().normalize().toString());
    }
}
