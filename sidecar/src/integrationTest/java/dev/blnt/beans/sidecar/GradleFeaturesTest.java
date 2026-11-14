package dev.blnt.beans.sidecar;

import dev.blnt.beans.gradle.model.JvmFeatureModel;
import dev.blnt.beans.gradle.model.ProjectModel;
import dev.blnt.beans.sidecar.gradle.GradleAdapter;
import dev.blnt.beans.sidecar.gradle.ImportParams;
import dev.blnt.beans.sidecar.model.Location;
import dev.blnt.beans.sidecar.model.ResourceFilter;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

class GradleFeaturesTest {
    @Test
    void features_preserve_their_source_set_links_and_configured_archive_paths(@TempDir Path root)
            throws Exception {
        root = root.toRealPath();
        Files.writeString(root.resolve("settings.gradle.kts"), "rootProject.name = \"features\"\n");
        Files.writeString(root.resolve("build.gradle.kts"), """
                plugins {
                    `java-library`
                    `java-test-fixtures`
                }
                val extraSources = sourceSets.create("extraSources") {
                    java.exclude("**/Excluded.java")
                }
                sourceSets.create("integrationTest")
                java.registerFeature("alternate") { usingSourceSet(extraSources) }
                tasks.named<Jar>("jar") {
                    archiveFileName.set("renamed-main.jar")
                    destinationDirectory.set(layout.projectDirectory.dir("artifacts"))
                }
                tasks.withType<Jar>().configureEach {
                    doLast { error("Import must not run JAR tasks") }
                }
                """);
        Files.createDirectories(root.resolve("src/extraSources/java"));
        Files.writeString(root.resolve("src/extraSources/java/Included.java"), "class Included {}");
        Files.writeString(root.resolve("src/extraSources/java/Excluded.java"), "class Excluded {}");

        var model = importModels(root).get(0);
        assertEquals(List.of("alternate", "main", "testFixtures"), model.getFeatures().stream()
                .map(JvmFeatureModel::getName).sorted().toList());
        var main = feature(model, "main");
        assertEquals("main", main.getSourceSetName());
        assertTrue(main.getArtifacts().contains(root.resolve("artifacts/renamed-main.jar").toString()));
        var alternate = feature(model, "alternate");
        assertEquals("extraSources", alternate.getSourceSetName());
        var sources = model.getSourceSets().stream()
                .filter(sourceSet -> sourceSet.getName().equals(alternate.getSourceSetName()))
                .findFirst().orElseThrow();
        assertEquals(List.of(root.resolve("src/extraSources/java/Included.java").toString()),
                sources.getSourceFiles());
        assertEquals("testFixtures", feature(model, "testFixtures").getSourceSetName());

        var modules = new GradleAdapter().importWorkspace(
                new ImportParams(root.toString(), System.getProperty("test.gradleHome")), progress -> {});
        var included = new ResourceFilter.File(sources.getSourceFiles().get(0));
        var association = modules.get(0).artifactSources().stream()
                .filter(value -> value.sources().contains(included)).findFirst().orElseThrow();
        assertTrue(association.artifacts().containsAll(
                alternate.getArtifacts().stream().map(Location::file).toList()));
        assertFalse(association.artifacts().contains(
                Location.file(root.resolve("artifacts/renamed-main.jar").toString())));

        for (var feature : model.getFeatures()) {
            for (String artifact : feature.getArtifacts()) {
                assertFalse(Files.exists(Path.of(artifact)), artifact);
            }
        }
        assertFalse(Files.exists(root.resolve("build/classes")));
    }

    @Test
    void artifacts_come_from_outgoing_configurations_including_secondary_variants(@TempDir Path root)
            throws Exception {
        root = root.toRealPath();
        Files.writeString(root.resolve("settings.gradle.kts"), "rootProject.name = \"published\"\n");
        Files.writeString(root.resolve("build.gradle.kts"), """
                plugins { java }
                val publishedJar = tasks.register<Jar>("publishedJar") {
                    from(sourceSets.main.get().output)
                    archiveFileName.set("public-api.jar")
                    destinationDirectory.set(layout.projectDirectory.dir("published"))
                }
                for (name in listOf("apiElements", "runtimeElements")) {
                    configurations.named(name) {
                        outgoing.artifacts.clear()
                        outgoing.artifact(publishedJar)
                    }
                }
                configurations.named("apiElements") {
                    outgoing.variants.create("additionalClasses") {
                        artifact(file("api-classes"))
                    }
                }
                configurations.named("runtimeElements") {
                    outgoing.variants.create("additionalResources") {
                        artifact(file("runtime-resources"))
                    }
                }
                """);

        var model = importModels(root).get(0);
        var artifacts = feature(model, "main").getArtifacts();
        String published = root.resolve("published/public-api.jar").toString();
        assertEquals(1L, artifacts.stream().filter(published::equals).count());
        assertTrue(artifacts.contains(root.resolve("api-classes").toString()));
        assertTrue(artifacts.contains(root.resolve("runtime-resources").toString()));
        assertFalse(artifacts.contains(root.resolve("build/libs/published.jar").toString()));
        var main = model.getSourceSets().stream()
                .filter(sourceSet -> sourceSet.getName().equals("main")).findFirst().orElseThrow();
        assertTrue(artifacts.containsAll(main.getClassesDirectories()));
        for (String artifact : artifacts) {
            assertFalse(Files.exists(Path.of(artifact)), artifact);
        }
    }

    @Test
    void project_dependency_artifacts_match_the_producing_feature_for_jars_and_directories(
            @TempDir Path root) throws Exception {
        root = root.toRealPath();
        Files.writeString(root.resolve("settings.gradle.kts"), """
                rootProject.name = "multi"
                include("app", "lib")
                """);
        Files.writeString(root.resolve("build.gradle.kts"), "");
        Files.createDirectories(root.resolve("app/src/main/java"));
        Files.createDirectories(root.resolve("lib/src/main/java"));
        Files.writeString(root.resolve("app/src/main/java/App.java"), "class App { Lib lib; }");
        Files.writeString(root.resolve("lib/src/main/java/Lib.java"), "class Lib {}");
        Files.writeString(root.resolve("app/build.gradle.kts"), """
                plugins { java }
                dependencies { implementation(project(":lib")) }
                """);

        for (String plugin : List.of("java", "java-library")) {
            Files.writeString(root.resolve("lib/build.gradle.kts"), "plugins { id(\"" + plugin + "\") }\n");
            var models = importModels(root);
            assertTrue(project(models, root).getFeatures().isEmpty());
            var app = project(models, root.resolve("app"));
            var lib = project(models, root.resolve("lib"));
            var classpath = app.getSourceSets().stream()
                    .filter(sourceSet -> sourceSet.getName().equals("main"))
                    .findFirst().orElseThrow().getCompileClasspath();
            assertFalse(classpath.isEmpty());
            assertTrue(feature(lib, "main").getArtifacts().containsAll(classpath));
            if (plugin.equals("java")) {
                assertEquals(List.of(root.resolve("lib/build/libs/lib.jar").toString()), classpath);
            } else {
                assertEquals(List.of(root.resolve("lib/build/classes/java/main").toString()), classpath);
            }
            assertFalse(Files.exists(root.resolve("lib/build")));
        }
    }

    private static List<ProjectModel> importModels(Path root) {
        return new GradleAdapter().importModel(
                new ImportParams(root.toString(), System.getProperty("test.gradleHome")), progress -> {});
    }

    private static ProjectModel project(List<ProjectModel> models, Path directory) {
        return models.stream().filter(model -> model.getProjectDirectory().equals(directory.toString()))
                .findFirst().orElseThrow();
    }

    private static JvmFeatureModel feature(ProjectModel project, String name) {
        return project.getFeatures().stream().filter(feature -> feature.getName().equals(name))
                .findFirst().orElseThrow();
    }
}
