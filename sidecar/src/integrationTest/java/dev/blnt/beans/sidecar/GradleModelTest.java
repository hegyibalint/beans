package dev.blnt.beans.sidecar;

import dev.blnt.beans.gradle.model.SourceSetModel;
import dev.blnt.beans.sidecar.gradle.GradleAdapter;
import dev.blnt.beans.sidecar.gradle.ImportParams;
import dev.blnt.beans.sidecar.model.ResourceFilter;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

class GradleModelTest {
    @Test
    void source_set_metadata_preserves_filters_and_missing_generated_roots(@TempDir Path root)
            throws Exception {
        root = root.toRealPath();
        Files.writeString(root.resolve("settings.gradle.kts"), "rootProject.name = \"custom\"\n");
        Files.writeString(root.resolve("build.gradle.kts"), """
                plugins { java }
                val generator = tasks.register("generateSources") {
                    outputs.dir("generated")
                    doLast { file("generator-ran").writeText("ran") }
                }
                sourceSets {
                    main { java.srcDir(generator) }
                    create("integrationTest") {
                        java.setSrcDirs(listOf("integrationSrc"))
                        java.exclude("**/Excluded.java")
                        compileClasspath = files("source-set.jar")
                    }
                }
                // The common model deliberately does not claim to describe task-specific overrides.
                tasks.named<JavaCompile>("compileIntegrationTestJava") {
                    classpath = files("task.jar")
                }
                """);
        Files.createDirectories(root.resolve("src/main/resources"));
        Files.writeString(root.resolve("src/main/resources/example.properties"), "key=value\n");
        Files.createDirectories(root.resolve("integrationSrc"));
        Files.writeString(root.resolve("integrationSrc/Included.java"), "class Included {}");
        Files.writeString(root.resolve("integrationSrc/Excluded.java"), "class Excluded {}");

        var models = new GradleAdapter().importModel(params(root), progress -> {});
        assertEquals(1, models.size());
        var main = sourceSet(models.get(0).getSourceSets(), "main");
        var custom = sourceSet(models.get(0).getSourceSets(), "integrationTest");
        assertTrue(main.getSourceDirectories().contains(root.resolve("generated").toString()));
        assertEquals(List.of(root.resolve("src/main/resources/example.properties").toString()),
                main.getSourceFiles());
        assertEquals(List.of(root.resolve("integrationSrc/Included.java").toString()),
                custom.getSourceFiles());
        assertEquals(List.of(root.resolve("source-set.jar").toString()), custom.getCompileClasspath());
        for (String directory : custom.getClassesDirectories()) {
            assertFalse(Files.exists(Path.of(directory)));
        }
        assertFalse(Files.exists(root.resolve("generator-ran")));

        var modules = new GradleAdapter().importWorkspace(params(root), progress -> {});
        var classpaths = modules.stream().flatMap(module -> module.classpaths().stream()).toList();
        var input = new ResourceFilter.File(custom.getSourceFiles().get(0));
        assertTrue(classpaths.stream().anyMatch(classpath -> classpath.inputs().contains(input)));
        var excluded = new ResourceFilter.File(root.resolve("integrationSrc/Excluded.java").toString());
        assertFalse(classpaths.stream().anyMatch(classpath -> classpath.inputs().contains(excluded)));
        assertFalse(modules.stream().flatMap(module -> module.artifactSources().stream())
                .anyMatch(association -> association.sources().contains(excluded)));
    }

    @Test
    void application_and_test_sources_include_java_kotlin_and_groovy(@TempDir Path root) throws Exception {
        root = root.toRealPath();
        Files.writeString(root.resolve("settings.gradle.kts"), "rootProject.name = \"mixed\"\n");
        Files.writeString(root.resolve("gradle.properties"), "kotlin.stdlib.default.dependency=false\n");
        Files.writeString(root.resolve("build.gradle.kts"), """
                plugins {
                    groovy
                    kotlin("jvm") version "2.4.21"
                }
                dependencies {
                    implementation(localGroovy())
                    runtimeOnly(files("runtime-only.jar"))
                }
                sourceSets.create("integrationTest")
                """);
        var languages = Map.of("java", "JavaExample.java", "kotlin", "KotlinExample.kt",
                "groovy", "GroovyExample.groovy");
        for (String name : List.of("main", "test", "integrationTest")) {
            for (var language : languages.entrySet()) {
                Path directory = Files.createDirectories(root.resolve("src/" + name + "/" + language.getKey()));
                String filename = language.getValue();
                String type = filename.substring(0, filename.lastIndexOf('.'));
                Files.writeString(directory.resolve(filename), "class " + type + " {}");
            }
        }

        var adapter = new GradleAdapter();
        var models = adapter.importModel(params(root), progress -> {});
        var sourceSets = models.get(0).getSourceSets();
        assertEquals(3, sourceSets.size());
        for (String name : List.of("main", "test", "integrationTest")) {
            var sources = sourceSet(sourceSets, name);
            assertEquals(3, sources.getSourceFiles().size());
            for (var language : languages.entrySet()) {
                assertTrue(sources.getSourceFiles().contains(root.resolve(
                        "src/" + name + "/" + language.getKey() + "/" + language.getValue()).toString()));
            }
            assertFalse(sources.getCompileClasspath().contains(root.resolve("runtime-only.jar").toString()));
        }

        var modules = adapter.importWorkspace(params(root), progress -> {});
        assertEquals(1, modules.size());
        var classpaths = modules.get(0).classpaths();
        assertEquals(3, classpaths.size());
        for (var sources : sourceSets) {
            var inputs = sources.getSourceFiles().stream()
                    .map(ResourceFilter.File::new).toList();
            assertTrue(classpaths.stream().anyMatch(classpath -> classpath.inputs().equals(inputs)));
        }
        var main = sourceSet(sourceSets, "main");
        var test = sourceSet(sourceSets, "test");
        assertTrue(test.getCompileClasspath().containsAll(main.getClassesDirectories()));
        assertFalse(Files.exists(root.resolve("build/classes")));
    }

    @Test
    void included_builds_are_not_automatically_imported(@TempDir Path root) throws Exception {
        root = root.toRealPath();
        Files.writeString(root.resolve("settings.gradle.kts"), """
                rootProject.name = "app"
                includeBuild("included")
                """);
        Files.writeString(root.resolve("build.gradle.kts"), """
                plugins { `java-library` }
                dependencies { implementation("sample:library:1.0") }
                """);
        Files.createDirectories(root.resolve("src/main/java"));
        Files.writeString(root.resolve("src/main/java/Use.java"), "class Use {}");
        Files.createDirectories(root.resolve("included/src/main/java"));
        Files.writeString(root.resolve("included/src/main/java/Target.java"), "class Target {}");
        Files.writeString(root.resolve("included/settings.gradle.kts"), "rootProject.name = \"library\"\n");
        Files.writeString(root.resolve("included/build.gradle.kts"), """
                plugins { `java-library` }
                group = "sample"
                version = "1.0"
                """);

        var models = new GradleAdapter().importModel(params(root), progress -> {});
        assertEquals(1, models.size());
        assertEquals(root.toString(), models.get(0).getProjectDirectory());
        assertFalse(Files.exists(root.resolve("included/build/classes")));
    }

    private static SourceSetModel sourceSet(List<SourceSetModel> sources, String name) {
        return sources.stream().filter(source -> source.getName().equals(name)).findFirst().orElseThrow();
    }

    private static ImportParams params(Path root) {
        return new ImportParams(root.toString(), System.getProperty("test.gradleHome"));
    }
}
